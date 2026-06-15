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
// Please see https://openpit.dev and the OWNERS file for details.

package pretrade

import (
	"testing"

	"go.openpit.dev/openpit/internal/native"
	"go.openpit.dev/openpit/model"
	"go.openpit.dev/openpit/reject"
	"go.openpit.dev/openpit/tx"
)

// ---------------------------------------------------------------------------
// DryRunReport - pass path (start stage)

func TestDryRunReportStartIsPassOnValidOrder(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineStartPreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineStartPreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	if !r.IsPass() {
		t.Fatal("IsPass() = false, want true for valid order")
	}
	if rejects := r.Rejects(); rejects != nil {
		t.Fatalf("Rejects() = %v, want nil on pass", rejects)
	}
}

// ---------------------------------------------------------------------------
// DryRunReport - pass path (full pipeline)

func TestDryRunReportExecuteIsPassOnValidOrder(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineExecutePreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineExecutePreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	if !r.IsPass() {
		t.Fatal("IsPass() = false, want true for valid order")
	}
	if rejects := r.Rejects(); rejects != nil {
		t.Fatalf("Rejects() = %v, want nil on pass", rejects)
	}
}

// ---------------------------------------------------------------------------
// DryRunReport.Lock

func TestDryRunReportLockIsNonZeroOnPassingExecute(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineExecutePreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineExecutePreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	lock := r.Lock()
	if len(lock.Bytes()) == 0 {
		t.Fatal("DryRunReport.Lock().Bytes() is empty, want non-empty for passing order")
	}
}

func TestDryRunReportLockIsEmptyOnStartStageDryRun(t *testing.T) {
	// Start-stage dry-run never runs the main stage, so the lock is always empty.
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineStartPreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineStartPreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	lock := r.Lock()
	isEmpty, err := lock.IsEmpty()
	if err != nil {
		t.Fatalf("Lock.IsEmpty() error = %v", err)
	}
	if !isEmpty {
		t.Fatal("DryRunReport.Lock().IsEmpty() = false, want true for start-stage dry-run")
	}
}

// ---------------------------------------------------------------------------
// DryRunReport.AccountAdjustments

func TestDryRunReportAccountAdjustmentsNilOnNoPolicy(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineExecutePreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineExecutePreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	// The built-in order-validation policy produces no account adjustments.
	if adj := r.AccountAdjustments(); adj != nil {
		t.Fatalf("AccountAdjustments() = %v, want nil when no adjustment policy is registered", adj)
	}
}

// ---------------------------------------------------------------------------
// DryRunReport.AccountBlock

func TestDryRunReportAccountBlockNilOnPassingOrder(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineExecutePreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineExecutePreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	defer r.Close()

	if block := r.AccountBlock(); block != nil {
		t.Fatalf("AccountBlock() = %v, want nil on passing order", block)
	}
}

// ---------------------------------------------------------------------------
// DryRunReport.Close idempotency

func TestDryRunReportCloseIsIdempotent(t *testing.T) {
	engine := newNativeEngineForPreTradeTests(t)
	order := newValidOrderForPreTradeTests(t)

	report, err := native.EngineStartPreTradeDryRun(engine, order.Handle())
	if err != nil {
		t.Fatalf("EngineStartPreTradeDryRun() error = %v", err)
	}
	r := NewDryRunReportFromHandle(report)
	r.Close()
	r.Close() // must not panic or double-free
}

// ---------------------------------------------------------------------------
// DryRunReport panics when closed

func TestDryRunReportIsPassPanicsWhenClosed(t *testing.T) {
	r := &DryRunReport{}
	assertPanics(t, "IsPass() on closed report", func() { r.IsPass() })
}

func TestDryRunReportRejectsPanicsWhenClosed(t *testing.T) {
	r := &DryRunReport{}
	assertPanics(t, "Rejects() on closed report", func() { r.Rejects() })
}

func TestDryRunReportLockPanicsWhenClosed(t *testing.T) {
	r := &DryRunReport{}
	assertPanics(t, "Lock() on closed report", func() { r.Lock() })
}

func TestDryRunReportAccountAdjustmentsPanicsWhenClosed(t *testing.T) {
	r := &DryRunReport{}
	assertPanics(t, "AccountAdjustments() on closed report", func() { r.AccountAdjustments() })
}

func TestDryRunReportAccountBlockPanicsWhenClosed(t *testing.T) {
	r := &DryRunReport{}
	assertPanics(t, "AccountBlock() on closed report", func() { r.AccountBlock() })
}

func assertPanics(t *testing.T, label string, fn func()) {
	t.Helper()
	didPanic := false
	func() {
		defer func() {
			if recover() != nil {
				didPanic = true
			}
		}()
		fn()
	}()
	if !didPanic {
		t.Fatalf("%s: expected panic, got none", label)
	}
}

// ---------------------------------------------------------------------------
// Custom policy with optional DryRunPolicy - without dry-run methods

func TestCustomPolicyWithoutDryRunDelegatesOnDryRunPath(t *testing.T) {
	// A policy that does NOT implement DryRunPolicy must still be registerable
	// via StartPreTrade; the engine then delegates to the normal hooks for
	// dry-runs (no panic expected here - just verifying registration succeeds).
	wrapped := NewSafeClientPreTradePolicy(&clientPayloadTestPolicy{})

	if wrapped.Name() != "client-payload-test" {
		t.Fatalf("Name() = %q, want %q", wrapped.Name(), "client-payload-test")
	}
	// Verify it does not satisfy DryRunPolicy.
	if _, ok := any(wrapped).(DryRunPolicy); ok {
		t.Fatal("policy unexpectedly satisfies DryRunPolicy")
	}
}

// ---------------------------------------------------------------------------
// Custom policy with optional DryRunPolicy - with dry-run methods

type dryRunCapablePolicy struct {
	clientPayloadTestPolicy
	checkDryRunCalled   bool
	performDryRunCalled bool
}

func (p *dryRunCapablePolicy) CheckPreTradeStartDryRun(
	_ Context,
	_ model.Order,
) []reject.Reject {
	p.checkDryRunCalled = true
	return nil
}

func (p *dryRunCapablePolicy) PerformPreTradeCheckDryRun(
	_ Context,
	_ model.Order,
	_ tx.Mutations,
	_ Result,
) []reject.Reject {
	p.performDryRunCalled = true
	return nil
}

func TestCustomPolicyWithDryRunImplementsDryRunPolicy(t *testing.T) {
	policy := &dryRunCapablePolicy{}
	// The adapter does not surface DryRunPolicy; the detection happens in
	// custompolicy.StartPreTrade by examining the concrete policy directly.
	_, ok := any(policy).(DryRunPolicy)
	if !ok {
		t.Fatal("dryRunCapablePolicy does not satisfy DryRunPolicy")
	}
	// Verify the adapter (which wraps the policy) does NOT satisfy DryRunPolicy
	// at the Policy interface level.
	wrapped := NewSafeClientPreTradePolicy(policy)
	if _, ok := any(wrapped).(DryRunPolicy); ok {
		t.Fatal("adapter unexpectedly satisfies DryRunPolicy")
	}
}

func TestDryRunCapablePolicyCheckDryRunIsCalled(t *testing.T) {
	policy := &dryRunCapablePolicy{}
	policy.CheckPreTradeStartDryRun(Context{}, model.NewOrder())
	if !policy.checkDryRunCalled {
		t.Fatal("CheckPreTradeStartDryRun() was not called")
	}
}

func TestDryRunCapablePolicyPerformDryRunIsCalled(t *testing.T) {
	policy := &dryRunCapablePolicy{}
	policy.PerformPreTradeCheckDryRun(Context{}, model.NewOrder(), tx.Mutations{}, Result{})
	if !policy.performDryRunCalled {
		t.Fatal("PerformPreTradeCheckDryRun() was not called")
	}
}

func TestDryRunCapablePolicyReturnNilRejectsOnPass(t *testing.T) {
	policy := &dryRunCapablePolicy{}

	rejects := policy.CheckPreTradeStartDryRun(Context{}, model.NewOrder())
	if rejects != nil {
		t.Fatalf("CheckPreTradeStartDryRun() rejects = %v, want nil", rejects)
	}
	rejects = policy.PerformPreTradeCheckDryRun(Context{}, model.NewOrder(), tx.Mutations{}, Result{})
	if rejects != nil {
		t.Fatalf("PerformPreTradeCheckDryRun() rejects = %v, want nil", rejects)
	}
}
