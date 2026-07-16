import { emit } from "./events.ts";
import { activePlayer, drawCard, type GameState } from "./state.ts";

function readyPhase(state: GameState): void {
  const player = activePlayer(state);
  for (const instance of player.play) {
    instance.exerted = false;
    instance.playedThisTurn = false;
  }
  for (const ink of player.inkwell) {
    ink.exerted = false;
  }
  player.inkedThisTurn = false;
}

function setPhase(state: GameState): void {
  emit("startOfTurn", { state, player: activePlayer(state) });
}

function drawPhase(state: GameState): void {
  const player = activePlayer(state);
  const isFirstTurnOfGame = state.turnNumber === 1 && state.activePlayer === 0;
  if (!isFirstTurnOfGame) {
    drawCard(player);
  }
}

/** Runs Ready -> Set -> Draw for the active player and leaves state.phase at "main". */
export function advanceToMain(state: GameState): void {
  state.phase = "ready";
  readyPhase(state);
  state.phase = "set";
  setPhase(state);
  state.phase = "draw";
  drawPhase(state);
  state.phase = "main";
}

/** Starts the whole game: runs turn 1's Ready/Set/Draw and lands on player 1's Main phase. */
export function startGame(state: GameState): void {
  advanceToMain(state);
}

/** Ends the active player's turn and advances into the next player's Ready/Set/Draw/Main. */
export function endTurn(state: GameState): void {
  emit("endOfTurn", { state, player: activePlayer(state) });
  state.activePlayer = state.activePlayer === 0 ? 1 : 0;
  if (state.activePlayer === 0) {
    state.turnNumber += 1;
  }
  advanceToMain(state);
}
