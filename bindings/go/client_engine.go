// Copyright The Pit Project Owners. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Please see https://github.com/openpitkit and the OWNERS file for details.

package openpit

import (
	"runtime"
	"runtime/cgo"
	"sync"

	"go.openpit.dev/openpit/internal/callback"
	"go.openpit.dev/openpit/internal/native"
	"go.openpit.dev/openpit/model"
	"go.openpit.dev/openpit/param"
	"go.openpit.dev/openpit/pkg/optional"
	"go.openpit.dev/openpit/pretrade"
	"go.openpit.dev/openpit/reject"
)

type ClientEngineOption func(*clientEngineOptions)

// UnsafeFastClientPayloadCallbacks selects callback adapters that trust every
// client payload reaching client policies to carry the builder's declared type.
//
// This mode removes safe adapter checks from every callback. A missing payload
// or a wrong payload type panics.
func UnsafeFastClientPayloadCallbacks() ClientEngineOption {
	return func(options *clientEngineOptions) {
		options.unsafeFastPayloadCallbacks = true
	}
}

type clientEngineOptions struct {
	unsafeFastPayloadCallbacks bool
}

type clientAccountAdjustment interface {
	EngineAccountAdjustment() model.AccountAdjustment
}

// ClientEngine runs the standard engine through client-owned order, execution
// report, and account-adjustment types.
//
// The ordinary Engine API remains the zero-payload fast path. ClientEngine is
// the opt-in path that allocates a cgo.Handle per submitted client payload so
// callbacks can receive the original typed value.
//
// Threading: ClientEngine follows the same threading contract as Engine.
// Payload handles allocated by the SDK (cgo.Handle wrapped around the client
// value) are released synchronously inside the same call that created them, so
// callers do not need to extend payload lifetime beyond that call.
type ClientEngine[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
	Adjustment clientAccountAdjustment,
] struct {
	engine *Engine
}

// Stop releases the underlying engine.
func (e *ClientEngine[Order, Report, Adjustment]) Stop() {
	e.engine.Stop()
}

// StartPreTrade runs the start stage with a client order payload.
//
// On accept, the returned ClientRequest owns the payload handle and releases it
// when Execute or Close is called. On reject or error, the handle is released
// before StartPreTrade returns.
func (e *ClientEngine[Order, Report, Adjustment]) StartPreTrade(
	order Order,
) (*ClientRequest, []reject.Reject, error) {
	engineOrder, payload := newClientOrderPayload(order)
	request, rejects, err := e.engine.StartPreTrade(engineOrder)
	runtime.KeepAlive(order)
	if err != nil || rejects != nil {
		payload.release()
		return nil, rejects, err
	}
	return newClientRequest(request, payload), nil, nil
}

// ExecutePreTrade runs the full pre-trade pipeline with a client order payload.
//
// The payload handle is released before ExecutePreTrade returns because all
// order callbacks have completed by then.
func (e *ClientEngine[Order, Report, Adjustment]) ExecutePreTrade(
	order Order,
) (*pretrade.Reservation, []reject.Reject, error) {
	engineOrder, payload := newClientOrderPayload(order)
	defer payload.release()
	reservation, rejects, err := e.engine.ExecutePreTrade(engineOrder)
	runtime.KeepAlive(order)
	return reservation, rejects, err
}

// ApplyExecutionReport applies a client execution report payload.
//
// The payload handle is released before ApplyExecutionReport returns because
// report callbacks are synchronous.
func (e *ClientEngine[Order, Report, Adjustment]) ApplyExecutionReport(
	report Report,
) (PostTradeResult, error) {
	engineReport, payload := newClientReportPayload(report)
	defer payload.release()
	result, err := e.engine.ApplyExecutionReport(engineReport)
	runtime.KeepAlive(report)
	return result, err
}

// ApplyAccountAdjustment applies client account-adjustment payloads.
//
// Payload handles are released before ApplyAccountAdjustment returns because
// account-adjustment callbacks are synchronous.
func (e *ClientEngine[Order, Report, Adjustment]) ApplyAccountAdjustment(
	accountID param.AccountID,
	adjustments []Adjustment,
) (optional.Option[reject.AccountAdjustmentBatchError], error) {
	engineAdjustments, payloads := newClientAdjustmentPayloads(adjustments)
	defer payloads.release()
	rejects, err := e.engine.ApplyAccountAdjustment(accountID, engineAdjustments)
	runtime.KeepAlive(adjustments)
	return rejects, err
}

//------------------------------------------------------------------------------
// ClientEngineBuilder

// ClientEngineBuilder is the initial stage of the client engine builder.
// Call one of FullSync, NoSync, or AccountSync to obtain a
// ClientSyncedEngineBuilder on which policies can be registered.
type ClientEngineBuilder[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
	Adjustment clientAccountAdjustment,
] struct {
	unsafeFastPayloadCallbacks bool
}

// NewClientEngineBuilder creates a builder for strategies that use custom
// order, execution report, and account-adjustment types.
//
// Call FullSync, NoSync, or AccountSync to select a sync policy
// and obtain a ClientSyncedEngineBuilder.
func NewClientEngineBuilder[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
	Adjustment clientAccountAdjustment,
](options ...ClientEngineOption) *ClientEngineBuilder[Order, Report, Adjustment] {
	config := clientEngineOptions{}
	for _, option := range options {
		option(&config)
	}
	return &ClientEngineBuilder[Order, Report, Adjustment]{
		unsafeFastPayloadCallbacks: config.unsafeFastPayloadCallbacks,
	}
}

// NewClientPreTradeEngineBuilder creates a client builder for custom order
// and execution report types while keeping account adjustments on the standard
// SDK model type.
func NewClientPreTradeEngineBuilder[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
](
	options ...ClientEngineOption,
) *ClientEngineBuilder[Order, Report, model.AccountAdjustment] {
	return NewClientEngineBuilder[Order, Report, model.AccountAdjustment](options...)
}

// NewClientAccountAdjustmentEngineBuilder creates a client builder for custom
// account-adjustment types while keeping orders and execution reports on the
// standard SDK model types.
func NewClientAccountAdjustmentEngineBuilder[
	Adjustment clientAccountAdjustment,
](
	options ...ClientEngineOption,
) *ClientEngineBuilder[model.Order, model.ExecutionReport, Adjustment] {
	return NewClientEngineBuilder[model.Order, model.ExecutionReport, Adjustment](options...)
}

// FullSync configures full thread-safety synchronization and returns a
// ClientSyncedEngineBuilder ready to accept policies.
func (
	b *ClientEngineBuilder[Order, Report, Adjustment],
) FullSync() *ClientSyncedEngineBuilder[Order, Report, Adjustment] {
	return &ClientSyncedEngineBuilder[Order, Report, Adjustment]{
		synced:                     NewEngineBuilder().FullSync(),
		unsafeFastPayloadCallbacks: b.unsafeFastPayloadCallbacks,
	}
}

// NoSync configures single-thread (no-sync) synchronization and returns
// a ClientSyncedEngineBuilder ready to accept policies.
func (
	b *ClientEngineBuilder[Order, Report, Adjustment],
) NoSync() *ClientSyncedEngineBuilder[Order, Report, Adjustment] {
	return &ClientSyncedEngineBuilder[Order, Report, Adjustment]{
		synced:                     NewEngineBuilder().NoSync(),
		unsafeFastPayloadCallbacks: b.unsafeFastPayloadCallbacks,
	}
}

// AccountSync configures account-sharded synchronization and returns a
// ClientSyncedEngineBuilder ready to accept policies. The resulting engine
// handle is safe for concurrent invocation when the caller pins each account
// to a single processing chain.
func (
	b *ClientEngineBuilder[Order, Report, Adjustment],
) AccountSync() *ClientSyncedEngineBuilder[Order, Report, Adjustment] {
	return &ClientSyncedEngineBuilder[Order, Report, Adjustment]{
		synced:                     NewEngineBuilder().AccountSync(),
		unsafeFastPayloadCallbacks: b.unsafeFastPayloadCallbacks,
	}
}

//------------------------------------------------------------------------------
// ClientSyncedEngineBuilder

// ClientSyncedEngineBuilder is the second stage of the client engine builder
// chain. Add at least one policy to advance to ClientReadyEngineBuilder where
// Build is available.
type ClientSyncedEngineBuilder[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
	Adjustment clientAccountAdjustment,
] struct {
	synced                     *SyncedEngineBuilder
	unsafeFastPayloadCallbacks bool
}

func (
	b *ClientSyncedEngineBuilder[Order, Report, Adjustment],
) newReady() *ClientReadyEngineBuilder[Order, Report, Adjustment] {
	return &ClientReadyEngineBuilder[Order, Report, Adjustment]{
		ready:                      newReadyEngineBuilder(b.synced),
		unsafeFastPayloadCallbacks: b.unsafeFastPayloadCallbacks,
	}
}

func (b *ClientSyncedEngineBuilder[Order, Report, Adjustment]) PreTrade(
	policy ...pretrade.ClientPreTradePolicy[Order, Report],
) *ClientReadyEngineBuilder[Order, Report, Adjustment] {
	rb := b.newReady()
	for _, p := range policy {
		rb.addPreTradePolicy(p)
	}
	return rb
}

// Builtin registers a built-in entity on the builder.
func (
	b *ClientSyncedEngineBuilder[Order, Report, Adjustment],
) Builtin(
	builtinReadyBuilder builtinReadyBuilder,
) *ClientReadyEngineBuilder[Order, Report, Adjustment] {
	return b.newReady().Builtin(builtinReadyBuilder)
}

//------------------------------------------------------------------------------
// ClientReadyEngineBuilder

// ClientReadyEngineBuilder is the third stage of the client engine builder
// chain. Accepts additional policies and builds the engine via Build.
type ClientReadyEngineBuilder[
	Order pretrade.ClientOrder,
	Report pretrade.ClientExecutionReport,
	Adjustment clientAccountAdjustment,
] struct {
	ready                      *ReadyEngineBuilder
	unsafeFastPayloadCallbacks bool
}

// Close releases the underlying builder and any policies it still owns.
func (b *ClientReadyEngineBuilder[Order, Report, Adjustment]) Close() {
	b.ready.Close()
}

// Build constructs a ClientEngine and transfers ownership of policies to it.
func (b *ClientReadyEngineBuilder[Order, Report, Adjustment]) Build() (
	*ClientEngine[Order, Report, Adjustment],
	error,
) {
	engine, err := b.ready.Build()
	if err != nil {
		return nil, err
	}
	return &ClientEngine[Order, Report, Adjustment]{engine: engine}, nil
}

func (b *ClientReadyEngineBuilder[Order, Report, Adjustment]) PreTrade(
	policy ...pretrade.ClientPreTradePolicy[Order, Report],
) *ClientReadyEngineBuilder[Order, Report, Adjustment] {
	for _, p := range policy {
		b.addPreTradePolicy(p)
	}
	return b
}

// Builtin registers a built-in entity on the builder.
func (
	b *ClientReadyEngineBuilder[Order, Report, Adjustment],
) Builtin(
	builtinReadyBuilder builtinReadyBuilder,
) *ClientReadyEngineBuilder[Order, Report, Adjustment] {
	b.ready.Builtin(builtinReadyBuilder)
	return b
}

func (b *ClientReadyEngineBuilder[Order, Report, Adjustment]) addPreTradePolicy(
	p pretrade.ClientPreTradePolicy[Order, Report],
) {
	if b.unsafeFastPayloadCallbacks {
		b.ready.PreTrade(pretrade.NewUnsafeFastClientPreTradePolicy(p))
		return
	}
	b.ready.PreTrade(pretrade.NewSafeClientPreTradePolicy(p))
}

// ClientRequest is a deferred pre-trade request that keeps the original client
// order payload alive until the request is executed or closed.
type ClientRequest struct {
	request *pretrade.Request
	payload *clientPayloadHandle
}

func newClientRequest(request *pretrade.Request, payload *clientPayloadHandle) *ClientRequest {
	return &ClientRequest{request: request, payload: payload}
}

// Close releases the request and the client order payload.
func (r *ClientRequest) Close() {
	if r.request != nil {
		r.request.Close()
		r.request = nil
	}
	r.payload.release()
}

// Execute runs the deferred pre-trade request and releases the client order
// payload after callbacks complete.
//
// Execute does not close the underlying request; call Close after Execute just
// as with a standard pretrade.Request.
func (r *ClientRequest) Execute() (*pretrade.Reservation, []reject.Reject, error) {
	reservation, rejects, err := r.request.Execute()
	r.payload.release()
	return reservation, rejects, err
}

type clientPayloadHandle struct {
	handle cgo.Handle
	once   sync.Once
}

func newClientPayloadHandle(value any) *clientPayloadHandle {
	return &clientPayloadHandle{handle: cgo.NewHandle(value)}
}

func (h *clientPayloadHandle) release() {
	if h == nil {
		return
	}
	h.once.Do(func() { h.handle.Delete() })
}

type clientPayloadHandles []*clientPayloadHandle

func (handles clientPayloadHandles) release() {
	for _, payload := range handles {
		payload.release()
	}
}

func newClientOrderPayload[Order pretrade.ClientOrder](
	order Order,
) (model.Order, *clientPayloadHandle) {
	engineOrder := order.EngineOrder()
	nativeOrder := engineOrder.Handle()
	payload := newClientPayloadHandle(order)
	native.OrderSetUserData(&nativeOrder, callback.NewUserDataFromHandle(payload.handle))
	return model.NewOrderFromHandle(nativeOrder), payload
}

func newClientReportPayload[Report pretrade.ClientExecutionReport](
	report Report,
) (model.ExecutionReport, *clientPayloadHandle) {
	engineReport := report.EngineExecutionReport()
	nativeReport := engineReport.Handle()
	payload := newClientPayloadHandle(report)
	native.ExecutionReportSetUserData(&nativeReport, callback.NewUserDataFromHandle(payload.handle))
	return model.NewExecutionReportFromHandle(nativeReport), payload
}

func newClientAdjustmentPayloads[Adjustment clientAccountAdjustment](
	adjustments []Adjustment,
) ([]model.AccountAdjustment, clientPayloadHandles) {
	engineAdjustments := make([]model.AccountAdjustment, len(adjustments))
	payloads := make(clientPayloadHandles, len(adjustments))
	for i, adjustment := range adjustments {
		engineAdjustments[i], payloads[i] = newClientAdjustmentPayload(adjustment)
	}
	return engineAdjustments, payloads
}

func newClientAdjustmentPayload[Adjustment clientAccountAdjustment](
	adjustment Adjustment,
) (model.AccountAdjustment, *clientPayloadHandle) {
	engineAdjustment := adjustment.EngineAccountAdjustment()
	nativeAdjustment := engineAdjustment.Handle()
	payload := newClientPayloadHandle(adjustment)
	native.AccountAdjustmentSetUserData(
		&nativeAdjustment,
		callback.NewUserDataFromHandle(payload.handle),
	)
	return model.NewAccountAdjustmentFromHandle(nativeAdjustment), payload
}
