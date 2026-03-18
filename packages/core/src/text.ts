/**
 * TextRenderable — styled text display with wrapping, truncation, alignment,
 * and inline styling spans.
 *
 * Extends the Renderable base class and feeds intrinsic size to the layout
 * engine so Taffy can measure text nodes correctly.
 */

import type { ComputedLayout, TextStyle } from "./types.js";
import { Renderable } from "./renderable.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ELLIPSIS = "\u2026";
const HALF_DIVISOR = 2;
const ZERO = 0;
const LAST_OFFSET = 1;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** How text wraps when it exceeds the available width. */
export type TextWrap = "word-wrap" | "break-word" | "no-wrap";

/** How text is truncated when it exceeds available space. */
export type TextOverflow = "clip" | "ellipsis";

/** How text is aligned within its container. */
export type TextAlign = "left" | "center" | "right";

/**
 * An inline styling span — applies a TextStyle to a range of text.
 *
 * Spans are index-based (UTF-16 code units, matching JS string indexing).
 * Overlapping spans are applied in order; later spans override earlier ones.
 */
export interface TextSpan {
  /** End index (exclusive). */
  end: number;
  /** Start index (inclusive). */
  start: number;
  /** Style to apply for this range. */
  style: TextStyle;
}

/** Options for constructing a TextRenderable. */
export interface TextOptions {
  align?: TextAlign;
  overflow?: TextOverflow;
  spans?: TextSpan[];
  text?: string;
  wrap?: TextWrap;
}

// ---------------------------------------------------------------------------
// Styled character — a character with its resolved style
// ---------------------------------------------------------------------------

/** A single character with its computed style after span resolution. */
export interface StyledChar {
  ch: string;
  style: TextStyle;
}

// ---------------------------------------------------------------------------
// Measurement result
// ---------------------------------------------------------------------------

/** Intrinsic size of a text block for layout. */
export interface TextMeasurement {
  /** Height in rows (number of lines after wrapping). */
  height: number;
  /** Width in columns (longest line after wrapping). */
  width: number;
}

/** Options for aligning a line of styled characters. */
export interface AlignLineOptions {
  align: TextAlign;
  fillStyle: TextStyle;
  line: StyledChar[];
  width: number;
}

/** Options for flushing a word onto the current line. */
interface FlushWordOptions {
  current: StyledChar[];
  lines: StyledChar[][];
  width: number;
  word: StyledChar[];
}

// ---------------------------------------------------------------------------
// Helper: merge two TextStyle objects (right overrides left)
// ---------------------------------------------------------------------------

const applyOverlayColorFg = (result: TextStyle, overlay: TextStyle): void => {
  if (overlay.fg !== undefined) {
    result.fg = overlay.fg;
  }
  if (overlay.bg !== undefined) {
    result.bg = overlay.bg;
  }
};

const applyOverlayColorDecor = (result: TextStyle, overlay: TextStyle): void => {
  if (overlay.underlineColor !== undefined) {
    result.underlineColor = overlay.underlineColor;
  }
  if (overlay.underlineStyle !== undefined) {
    result.underlineStyle = overlay.underlineStyle;
  }
};

const applyOverlayFlagsPrimary = (result: TextStyle, overlay: TextStyle): void => {
  if (overlay.bold !== undefined) {
    result.bold = overlay.bold;
  }
  if (overlay.dim !== undefined) {
    result.dim = overlay.dim;
  }
  if (overlay.italic !== undefined) {
    result.italic = overlay.italic;
  }
  if (overlay.underline !== undefined) {
    result.underline = overlay.underline;
  }
};

const applyOverlayFlagsSecondary = (result: TextStyle, overlay: TextStyle): void => {
  if (overlay.strikethrough !== undefined) {
    result.strikethrough = overlay.strikethrough;
  }
  if (overlay.overline !== undefined) {
    result.overline = overlay.overline;
  }
  if (overlay.blink !== undefined) {
    result.blink = overlay.blink;
  }
  if (overlay.reverse !== undefined) {
    result.reverse = overlay.reverse;
  }
};

const mergeTextStyle = (base: TextStyle, overlay: TextStyle): TextStyle => {
  const result: TextStyle = { ...base };
  applyOverlayColorFg(result, overlay);
  applyOverlayColorDecor(result, overlay);
  applyOverlayFlagsPrimary(result, overlay);
  applyOverlayFlagsSecondary(result, overlay);
  return result;
};

// ---------------------------------------------------------------------------
// Helper: resolve spans to per-character styles
// ---------------------------------------------------------------------------

/** Build a StyledChar array from text with a base style. */
const buildStyledChars = (text: string, baseStyle: TextStyle): StyledChar[] =>
  Array.from(text, (ch) => ({ ch, style: { ...baseStyle } }));

/**
 * Resolve inline spans to produce a StyledChar array for the entire text.
 *
 * The base style is applied to every character, then each span overlays its
 * style on the matching range. Spans are applied in order.
 */
export const resolveSpans = (
  text: string,
  baseStyle: TextStyle,
  spans: TextSpan[],
): StyledChar[] => {
  const chars = buildStyledChars(text, baseStyle);

  for (const span of spans) {
    const start = Math.max(ZERO, span.start);
    const end = Math.min(text.length, span.end);
    for (let idx = start; idx < end; idx++) {
      chars[idx].style = mergeTextStyle(chars[idx].style, span.style);
    }
  }

  return chars;
};

// ---------------------------------------------------------------------------
// Helper: word-boundary detection
// ---------------------------------------------------------------------------

const isWhitespace = (ch: string): boolean => ch === " " || ch === "\t";

// ---------------------------------------------------------------------------
// Wrapping: split on hard newlines
// ---------------------------------------------------------------------------

const splitOnNewlines = (chars: StyledChar[]): StyledChar[][] => {
  const segments: StyledChar[][] = [];
  let seg: StyledChar[] = [];
  for (const sc of chars) {
    if (sc.ch === "\n") {
      segments.push(seg);
      seg = [];
    } else {
      seg.push(sc);
    }
  }
  segments.push(seg);
  return segments;
};

// ---------------------------------------------------------------------------
// Wrapping: break-word mode
// ---------------------------------------------------------------------------

const wrapBreakWord = (
  segment: StyledChar[],
  width: number,
  lines: StyledChar[][],
): void => {
  let current: StyledChar[] = [];
  for (const sc of segment) {
    current.push(sc);
    if (current.length >= width) {
      lines.push(current);
      current = [];
    }
  }
  lines.push(current);
};

// ---------------------------------------------------------------------------
// Wrapping: word-wrap helpers
// ---------------------------------------------------------------------------

const breakLongWord = (
  word: StyledChar[],
  width: number,
  lines: StyledChar[][],
): StyledChar[] => {
  let current: StyledChar[] = [];
  let pos = ZERO;
  while (pos < word.length) {
    const chunk = word.slice(pos, pos + width);
    if (pos + width < word.length) {
      lines.push(chunk);
    } else {
      current = chunk;
    }
    pos += width;
  }
  return current;
};

const flushWord = (opts: FlushWordOptions): StyledChar[] => {
  const { word, width, current, lines } = opts;
  if (current.length + word.length > width) {
    if (current.length > ZERO) {
      lines.push(current);
    }
    if (word.length > width) {
      return breakLongWord(word, width, lines);
    }
    return [...word];
  }
  current.push(...word);
  return current;
};

/** State tracked during word-wrap processing. */
interface WordWrapState {
  current: StyledChar[];
  lines: StyledChar[][];
  pos: number;
  segment: StyledChar[];
  width: number;
}

const consumeWhitespace = (state: WordWrapState): void => {
  while (state.pos < state.segment.length && isWhitespace(state.segment[state.pos].ch)) {
    if (state.current.length < state.width) {
      state.current.push(state.segment[state.pos]);
    } else {
      state.lines.push(state.current);
      state.current = [];
    }
    state.pos++;
  }
};

const collectAndFlushWord = (state: WordWrapState): void => {
  const word: StyledChar[] = [];
  while (state.pos < state.segment.length && !isWhitespace(state.segment[state.pos].ch)) {
    word.push(state.segment[state.pos]);
    state.pos++;
  }
  if (word.length > ZERO) {
    state.current = flushWord({ current: state.current, lines: state.lines, width: state.width, word });
  }
};

const wrapWordWrap = (
  segment: StyledChar[],
  width: number,
  lines: StyledChar[][],
): void => {
  const state: WordWrapState = { current: [], lines, pos: ZERO, segment, width };

  while (state.pos < segment.length) {
    consumeWhitespace(state);
    collectAndFlushWord(state);
  }

  lines.push(state.current);
};

// ---------------------------------------------------------------------------
// Wrapping: main entry
// ---------------------------------------------------------------------------

const getEffectiveWidth = (maxWidth: number | undefined): number | undefined => {
  if (maxWidth !== undefined && maxWidth > ZERO) {
    return maxWidth;
  }
  return undefined;
};

/**
 * Wrap styled characters into lines based on wrap mode and available width.
 *
 * Returns an array of lines, each being an array of StyledChar.
 * If `maxWidth` is 0 or undefined, no wrapping is performed (equivalent to
 * "no-wrap" except hard newlines are still honoured).
 */
export const wrapText = (
  chars: StyledChar[],
  wrap: TextWrap,
  maxWidth: number | undefined,
): StyledChar[][] => {
  const segments = splitOnNewlines(chars);
  const effectiveWidth = getEffectiveWidth(maxWidth);
  const lines: StyledChar[][] = [];

  for (const segment of segments) {
    if (effectiveWidth === undefined || wrap === "no-wrap") {
      lines.push(segment);
    } else if (wrap === "break-word") {
      wrapBreakWord(segment, effectiveWidth, lines);
    } else {
      // Word-wrap: break at word boundaries, fallback to break-word for long words
      wrapWordWrap(segment, effectiveWidth, lines);
    }
  }

  return lines;
};

// ---------------------------------------------------------------------------
// Truncation
// ---------------------------------------------------------------------------

/**
 * Truncate a line of styled characters to fit within maxWidth.
 *
 * When overflow is "ellipsis", the last visible character is replaced with
 * the ellipsis character if truncation occurs.
 */
export const truncateLine = (
  line: StyledChar[],
  maxWidth: number,
  overflow: TextOverflow,
): StyledChar[] => {
  if (line.length <= maxWidth) {
    return line;
  }

  if (overflow === "ellipsis" && maxWidth > ZERO) {
    const truncated = line.slice(ZERO, maxWidth);
    const lastIdx = truncated.length - LAST_OFFSET;
    const lastStyle = truncated[lastIdx].style;
    truncated[lastIdx] = { ch: ELLIPSIS, style: lastStyle };
    return truncated;
  }

  return line.slice(ZERO, maxWidth);
};

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

/**
 * Pad a line to the given width according to alignment.
 *
 * Adds space-styled characters on the left/right as needed. If the line is
 * already wider than the target, it is returned unchanged.
 */
export const alignLine = (opts: AlignLineOptions): StyledChar[] => {
  const { align, fillStyle, line, width } = opts;

  if (line.length >= width || align === "left") {
    return line;
  }

  const gap = width - line.length;
  const makeSpace = (): StyledChar => ({ ch: " ", style: fillStyle });

  if (align === "right") {
    return [...Array.from({ length: gap }, makeSpace), ...line];
  }

  // Center alignment
  const leftPad = Math.floor(gap / HALF_DIVISOR);
  const rightPad = gap - leftPad;
  return [
    ...Array.from({ length: leftPad }, makeSpace),
    ...line,
    ...Array.from({ length: rightPad }, makeSpace),
  ];
};

// ---------------------------------------------------------------------------
// Text measurement
// ---------------------------------------------------------------------------

/**
 * Measure the intrinsic size of text given wrapping constraints.
 *
 * This produces a width/height pair that can be fed to Taffy as intrinsic
 * size hints for the layout node.
 */
export const measureText = (
  text: string,
  wrap: TextWrap,
  maxWidth?: number,
): TextMeasurement => {
  if (text.length === ZERO) {
    return { height: ZERO, width: ZERO };
  }

  const chars = buildStyledChars(text, {});
  const lines = wrapText(chars, wrap, maxWidth);

  let longestLine = ZERO;
  for (const line of lines) {
    if (line.length > longestLine) {
      longestLine = line.length;
    }
  }

  return { height: lines.length, width: longestLine };
};

// ---------------------------------------------------------------------------
// TextRenderable
// ---------------------------------------------------------------------------

export class TextRenderable extends Renderable {
  private _wrap: TextWrap = "word-wrap";
  private _overflow: TextOverflow = "clip";
  private _align: TextAlign = "left";
  private _spans: TextSpan[] = [];

  constructor(options?: TextOptions) {
    super();
    if (options) {
      this.applyOptions(options);
    }
  }

  private applyOptions(options: TextOptions): void {
    if (options.text !== undefined) {
      this.setText(options.text);
    }
    if (options.wrap !== undefined) {
      this._wrap = options.wrap;
    }
    if (options.overflow !== undefined) {
      this._overflow = options.overflow;
    }
    if (options.align !== undefined) {
      this._align = options.align;
    }
    if (options.spans !== undefined) {
      this._spans = options.spans;
    }
  }

  // -----------------------------------------------------------------------
  // Wrap
  // -----------------------------------------------------------------------

  get wrap(): TextWrap {
    return this._wrap;
  }

  setWrap(wrap: TextWrap): void {
    if (this._wrap !== wrap) {
      this._wrap = wrap;
      this.markDirty();
    }
  }

  // -----------------------------------------------------------------------
  // Overflow
  // -----------------------------------------------------------------------

  get overflow(): TextOverflow {
    return this._overflow;
  }

  setOverflow(overflow: TextOverflow): void {
    if (this._overflow !== overflow) {
      this._overflow = overflow;
      this.markDirty();
    }
  }

  // -----------------------------------------------------------------------
  // Alignment
  // -----------------------------------------------------------------------

  get align(): TextAlign {
    return this._align;
  }

  setAlign(align: TextAlign): void {
    if (this._align !== align) {
      this._align = align;
      this.markDirty();
    }
  }

  // -----------------------------------------------------------------------
  // Spans
  // -----------------------------------------------------------------------

  get spans(): readonly TextSpan[] {
    return this._spans;
  }

  setSpans(spans: TextSpan[]): void {
    this._spans = spans;
    this.markDirty();
  }

  addSpan(span: TextSpan): void {
    this._spans.push(span);
    this.markDirty();
  }

  clearSpans(): void {
    if (this._spans.length > ZERO) {
      this._spans = [];
      this.markDirty();
    }
  }

  // -----------------------------------------------------------------------
  // Measurement — intrinsic size for Taffy layout
  // -----------------------------------------------------------------------

  /**
   * Measure text intrinsic size for layout.
   *
   * If maxWidth is provided, text is wrapped to that width. Otherwise the
   * natural (unwrapped) width is returned.
   */
  measure(maxWidth?: number): TextMeasurement {
    const content = this.text ?? "";
    return measureText(content, this._wrap, maxWidth);
  }

  // -----------------------------------------------------------------------
  // Rendering — produce styled character lines for output
  // -----------------------------------------------------------------------

  private renderLine(line: StyledChar[], width: number, baseStyle: TextStyle): StyledChar[] {
    let processed = truncateLine(line, width, this._overflow);
    processed = alignLine({ align: this._align, fillStyle: baseStyle, line: processed, width });
    return processed;
  }

  private wrapAndProcess(content: string, width: number | undefined): StyledChar[][] {
    const baseStyle = this.textStyle;
    const styledChars = resolveSpans(content, baseStyle, this._spans);
    const wrappedLines = wrapText(styledChars, this._wrap, width);

    if (width === undefined || width <= ZERO) {
      return wrappedLines;
    }

    return wrappedLines.map((line) => this.renderLine(line, width, baseStyle));
  }

  /**
   * Render the text into lines of styled characters, applying wrapping,
   * truncation, alignment, and inline spans.
   *
   * If a ComputedLayout is available (from the last layout pass), its width
   * is used as the constraint. An explicit maxWidth overrides the layout.
   */
  render(maxWidth?: number): StyledChar[][] {
    const content = this.text ?? "";
    if (content.length === ZERO) {
      return [];
    }

    const width = maxWidth ?? this.layout?.width;
    return this.wrapAndProcess(content, width);
  }

  // -----------------------------------------------------------------------
  // Lifecycle
  // -----------------------------------------------------------------------

  override onLayout(layout: ComputedLayout): void {
    // Re-measure is implicit — layout width drives wrapping in render().
    super.onLayout(layout);
  }
}
