// Loads the WebAssembly build (npm/pkg, built by `pnpm wasm:build`) and
// converts between JS values and the JSON the Rust exports take and return.
// The conventions are described in crates/federation-composition-wasm/src/lib.rs.
import { GraphQLError, Location, type DocumentNode } from "graphql";
import { createRequire } from "node:module";

type Bindings = typeof import("../pkg/federation_composition_wasm.js");

const require = createRequire(import.meta.url);
let gluePath: string;
try {
  gluePath = require.resolve("../pkg/federation_composition_wasm.js");
} catch {
  throw new Error(
    "@lemonberrylabs/federation-composition-wasm: pkg/ is missing; build the WebAssembly bindings with `pnpm wasm:build`",
  );
}

let instance: Bindings | undefined;

/** The bindings; a fresh instance after a panic trapped the previous one. */
function bindings(): Bindings {
  if (instance === undefined) {
    // The nodejs glue instantiates the module when it is evaluated.
    delete require.cache[gluePath];
    instance = require(gluePath) as Bindings;
  }
  return instance;
}

type Fn = (...args: never[]) => unknown;
type FnKeys = {
  [K in keyof Bindings]: Bindings[K] extends Fn ? K : never;
}[keyof Bindings];

/** Calls a Rust export, rethrowing what it throws as the JS error the TS library throws. */
export function call<K extends FnKeys>(
  name: K,
  ...args: Parameters<Bindings[K]>
): ReturnType<Bindings[K]> {
  const current = bindings();
  try {
    return (current[name] as (...a: unknown[]) => ReturnType<Bindings[K]>)(
      ...args,
    );
  } catch (error) {
    throw toJsError(error, current);
  }
}

/** A Rust panic, surfaced as a JS `Error`. */
export interface RustPanicError extends Error {
  rustPanicLocation?: string;
}

function toJsError(error: unknown, current: Bindings): unknown {
  if (typeof error === "string") {
    // A `ThrownError` JSON from Rust.
    const thrown = JSON.parse(error) as
      | { kind: "error"; name: string; message: string }
      | {
          kind: "graphql";
          message: string;
          extensions?: Record<string, unknown>;
        };
    if (thrown.kind === "graphql") {
      return new GraphQLError(thrown.message, {
        extensions: thrown.extensions,
      });
    }
    return createError(thrown.name, thrown.message);
  }
  if (error instanceof WebAssembly.RuntimeError) {
    // A panic aborts with a trap; the instance's state (its stack pointer,
    // anything mid-update) cannot be trusted afterwards, so the next call
    // loads a new one.
    let panic: string | undefined;
    try {
      panic = current.take_panic_message();
    } catch {}
    instance = undefined;
    if (panic === undefined) {
      return error;
    }
    const { message, location } = JSON.parse(panic) as {
      message: string;
      location?: string;
    };
    const rustError: RustPanicError = new Error(message, { cause: error });
    rustError.rustPanicLocation = location;
    return rustError;
  }
  // A JS exception thrown by a callback passed to Rust propagates unchanged.
  return error;
}

/** `new Error(message)`, or a runtime error of the named built-in class (`TypeError`...). */
function createError(name: string, message: string): Error {
  const ErrorClass = (globalThis as Record<string, unknown>)[name];
  if (
    typeof ErrorClass === "function" &&
    (ErrorClass === Error || ErrorClass.prototype instanceof Error)
  ) {
    return new (ErrorClass as ErrorConstructor)(message);
  }
  const error = new Error(message);
  error.name = name;
  return error;
}

const dropLoc = (key: string, v: unknown) => (key === "loc" ? undefined : v);
const locationPrototype = Location.prototype as { toJSON?: unknown };
const locationToJSON = locationPrototype.toJSON;
const omitLocation = () => undefined;

/** JSON of an AST node (or an object containing AST nodes), without `loc`. */
export function toJSON(value: unknown): string {
  // PERF: a replacer function runs for every key and halves JSON.stringify's speed. Instead,
  // while this synchronous call runs, graphql's `Location#toJSON` returns `undefined`, so
  // JSON.stringify leaves out every `loc` that holds a `Location` (what `parse` puts there).
  // Any other `loc` value (a plain `{ start, end }` from a JSON round trip, or a `Location` of
  // another copy of graphql) is still written, and the Rust deserializer ignores `loc` keys.
  // If the prototype cannot be patched (frozen), the replacer does the same job.
  try {
    locationPrototype.toJSON = omitLocation;
  } catch {
    return JSON.stringify(value, dropLoc);
  }
  try {
    if (locationPrototype.toJSON !== omitLocation) {
      return JSON.stringify(value, dropLoc);
    }
    return JSON.stringify(value);
  } finally {
    locationPrototype.toJSON = locationToJSON;
  }
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/**
 * {@link toJSON} as UTF-8 bytes, for the exports that take documents (`&[u8]`): passing a
 * `Uint8Array` is a copy, while wasm-bindgen re-encodes a string character by character.
 */
export function encodeJSON(value: unknown): Uint8Array {
  return encoder.encode(toJSON(value));
}

export function parseJSON<T>(json: string): T {
  return JSON.parse(json) as T;
}

/** {@link parseJSON} of UTF-8 bytes returned by an export (`Vec<u8>`). */
export function decodeJSON<T>(json: Uint8Array): T {
  return JSON.parse(decoder.decode(json)) as T;
}

export interface GraphQLErrorJSON {
  message: string;
  extensions?: Record<string, unknown>;
}

/** Errors that are results (composition errors), as `GraphQLError`s. */
export function toGraphQLErrors(errors: GraphQLErrorJSON[]): GraphQLError[] {
  return errors.map(
    (e) => new GraphQLError(e.message, { extensions: e.extensions }),
  );
}

type Service = {
  name: string;
  url?: string;
  typeDefs: DocumentNode;
};

/** A `ServiceDefinition` as JSON, with `name` converted the way `validate` does (`String(name)`). */
function serviceJSONValue(s: Service) {
  return { name: String(s.name), url: s.url, typeDefs: s.typeDefs };
}

/** The JSON of a `ServiceDefinition`, as bytes ({@link encodeJSON}). */
export function serviceToJSON(service: Service): Uint8Array {
  return encodeJSON(serviceJSONValue(service));
}

/** The JSON of `ServiceDefinition`s, as bytes ({@link encodeJSON}). */
export function servicesToJSON(services: ReadonlyArray<Service>): Uint8Array {
  return encodeJSON(services.map(serviceJSONValue));
}
