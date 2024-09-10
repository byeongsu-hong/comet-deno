const { core } = Deno;
const { ops } = core;

function argsToMessage(...args) {
  return args.map((arg) => JSON.stringify(arg)).join(" ");
}

const console = {
  log: (...args) => core.print(`[log]: ${argsToMessage(...args)}\n`, false),
  info: (...args) => core.print(`[info]: ${argsToMessage(...args)}\n`, false),
  error: (...args) => core.print(`[err]: ${argsToMessage(...args)}\n`, true),
  warn: (...args) => core.print(`[warn]: ${argsToMessage(...args)}\n`, true),
  debug: (...args) => core.print(`[debug]: ${argsToMessage(...args)}\n`, true),
  assert: (condition, ...args) => {
    if (!condition) {
      core.print(`[assert]: ${argsToMessage(...args)}\n`, true);
    }
  },
  time: (label) => {
    console._timers = console._timers || {};
    console._timers[label] = Date.now();
  },
  timeEnd: (label) => {
    if (console._timers && console._timers[label]) {
      const duration = Date.now() - console._timers[label];
      core.print(`[time]: ${label}: ${duration}ms\n`, false);
      delete console._timers[label];
    }
  },
  trace: (...args) => {
    const err = new Error();
    err.name = "Trace";
    err.message = argsToMessage(...args);
    core.print(err.stack + "\n", true);
  },
};

/**
 * A key-value store interface.
 * @namespace store
 */
const store = {
  /**
   * Sets a value for a given key in the store.
   * @function
   * @param {string} k - The key to set.
   * @param {string} v - The value to set.
   * @returns {Promise<string>} A promise that resolves to the set value.
   * @throws {Error} If there's an issue accessing the store or if the key is invalid.
   */
  set: (k, v) => ops.op_kv_set(k, v),

  /**
   * Gets the value for a given key from the store.
   * @function
   * @param {string} k - The key to get.
   * @returns {Promise<string>} A promise that resolves to the value associated with the key, or an empty string if not found.
   * @throws {Error} If there's an issue accessing the store or if the key is invalid.
   */
  get: (k) => ops.op_kv_get(k),
};

const context = {
  /**
   * Emits an event with attributes
   * @param {{
   *    type: string;
   *    attributes: {
   *      key: string;
   *      value: string;
   *      index: boolean
   *    }[];
   *  }} event - Event object to emit.
   */
  emit: (event) => ops.op_ctx_emit(event),

  /**
   * Sends a response message.
   * @param {object} response - The message to send.
   * @returns {Promise<void>} A promise that resolves when the message is sent.
   */
  respond: (response) => ops.op_ctx_respond(response),

  /**
   * Retrieves the sender of the current request.
   * @returns {string} The sender of the current request.
   */
  getSender: () => ops.op_ctx_get_sender(),

  /**
   * Fetches a request object.
   * @returns {T} The request object.
   * @template T
   */
  getRequest: () => ops.op_ctx_get_request(),
};

globalThis.console = console;
globalThis.store = store;
globalThis.context = context;
