export type DictationState = "idle" | "listening" | "transcribing-segment";

export interface DictationMachine {
  state: DictationState;
  pendingSegment: string;
  setState: (s: DictationState) => void;
  setPending: (t: string) => void;
  reset: () => void;
}

export function createMachine(): DictationMachine {
  let state: DictationState = "idle";
  let pendingSegment = "";
  return {
    get state() { return state; },
    get pendingSegment() { return pendingSegment; },
    setState(s) { state = s; },
    setPending(t) { pendingSegment = t; },
    reset() { state = "idle"; pendingSegment = ""; },
  };
}
