export type EventName =
  | "startOfTurn"
  | "endOfTurn"
  | "play"
  | "quest"
  | "challenge"
  | "banish"
  | "ink";

// Payload shape is intentionally loose until phase 4 abilities need specific fields.
// biome-ignore lint/suspicious/noExplicitAny: see above
type Handler = (payload: any) => void;

const handlers: Partial<Record<EventName, Handler[]>> = {};

export function on(event: EventName, handler: Handler): void {
  (handlers[event] ??= []).push(handler);
}

export function emit(event: EventName, payload: unknown): void {
  for (const handler of handlers[event] ?? []) {
    handler(payload);
  }
}

/** Test-only: clears all registered handlers between isolated test runs. */
export function clearHandlers(): void {
  for (const key of Object.keys(handlers)) {
    delete handlers[key as EventName];
  }
}
