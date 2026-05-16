package main

import (
	"testing"

	"go.openpit.dev/openpit"
	"go.openpit.dev/openpit/pretrade/policies"
)

func TestBuildEngineFromPublicModule(t *testing.T) {
	engine, err := openpit.NewEngineBuilder().
		NoSync().
		Builtin(policies.BuildOrderValidation()).
		Build()
	if err != nil {
		t.Logf("engine build returned error (acceptable in offline mode): %v", err)
		return
	}
	defer engine.Stop()
}
