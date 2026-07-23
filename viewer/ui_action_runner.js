function normalizeError(error) {
  return error instanceof Error ? error : new Error(String(error));
}

export function createUiActionRunner(options = {}) {
  async function reportFailure(scope, error, recoveryError = null, metadata = null) {
    const normalized = normalizeError(error);
    const recovery = recoveryError ? normalizeError(recoveryError) : null;
    const detail = {
      scope,
      message: normalized.message,
      recoveryMessage: recovery?.message || null,
      metadata,
    };
    console.error(`[chemsema] UI action '${scope}' failed`, normalized);
    if (recovery) {
      console.error(`[chemsema] UI action '${scope}' recovery failed`, recovery);
    }
    options.notify?.(
      recovery
        ? `${normalized.message} (recovery failed: ${recovery.message})`
        : normalized.message,
    );
    if (typeof globalThis.dispatchEvent === "function"
      && typeof globalThis.CustomEvent === "function") {
      globalThis.dispatchEvent(new globalThis.CustomEvent("chemsema-ui-action-error", {
        detail,
      }));
    }
    if (options.trace) {
      try {
        await options.trace("uiAction.failed", {
          ...detail,
          error: normalized,
          recoveryError: recovery,
        });
      } catch (traceError) {
        console.error(`[chemsema] UI action '${scope}' failure tracing failed`, traceError);
      }
    }
    options.onFailure?.({ ...detail, error: normalized, recoveryError: recovery });
    return normalized;
  }

  async function execute(scope, action, actionOptions, rethrow) {
    try {
      return await action();
    } catch (error) {
      if (options.isAbortError?.(error)) {
        return undefined;
      }
      let recoveryError = null;
      if (actionOptions?.recover) {
        try {
          await actionOptions.recover(error);
        } catch (caughtRecoveryError) {
          recoveryError = caughtRecoveryError;
        }
      }
      const normalized = await reportFailure(
        scope,
        error,
        recoveryError,
        actionOptions?.metadata || null,
      );
      if (rethrow) {
        throw normalized;
      }
      return undefined;
    }
  }

  function run(scope, action, actionOptions = {}) {
    return execute(scope, action, actionOptions, true);
  }

  function listener(scope, handler, actionOptions = {}) {
    return (event) => {
      void start(scope, () => handler(event), actionOptions);
    };
  }

  function start(scope, action, actionOptions = {}) {
    return execute(scope, action, actionOptions, false);
  }

  return { run, start, listener };
}
