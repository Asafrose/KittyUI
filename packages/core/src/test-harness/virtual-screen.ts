/**
 * VirtualScreen — in-memory terminal grid with a minimal ANSI parser.
 *
 * Only handles CUP (cursor position) and SGR (select graphic rendition) —
 * those are the only sequences emitted by DoubleBuffer.diff().
 */

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ESC = 0x1b;
const OPEN_BRACKET = 0x5b; // '['
const SGR_RESET = 0;
const SGR_BOLD = 1;
const SGR_ITALIC = 3;
const SGR_FG_EXTENDED = 38;
const SGR_BG_EXTENDED = 48;
const SGR_RGB_INDICATOR = 2;
const HEX_RADIX = 16;
const BYTE_PAD_LEN = 2;

// ---------------------------------------------------------------------------
// Cell type
// ---------------------------------------------------------------------------

interface ScreenCell {
  ch: string;
  fg: string | undefined;
  bg: string | undefined;
  bold: boolean;
  italic: boolean;
}

const defaultCell = (): ScreenCell => ({
  bg: undefined,
  bold: false,
  ch: " ",
  fg: undefined,
  italic: false,
});

// ---------------------------------------------------------------------------
// VirtualScreen
// ---------------------------------------------------------------------------

export class VirtualScreen {
  readonly cols: number;
  readonly rows: number;
  private grid: ScreenCell[][];

  // Cursor position (0-based)
  private cursorRow = 0;
  private cursorCol = 0;

  // Current style state
  private currentFg: string | undefined;
  private currentBg: string | undefined;
  private currentBold = false;
  private currentItalic = false;

  constructor(cols: number, rows: number) {
    this.cols = cols;
    this.rows = rows;
    this.grid = [];
    for (let r = 0; r < rows; r++) {
      const row: ScreenCell[] = [];
      for (let c = 0; c < cols; c++) {
        row.push(defaultCell());
      }
      this.grid.push(row);
    }
  }

  /** Apply raw ANSI output to the virtual screen. */
  apply(data: Uint8Array): void {
    let i = 0;
    while (i < data.length) {
      if (data[i] === ESC && i + 1 < data.length && data[i + 1] === OPEN_BRACKET) {
        // Start of CSI sequence
        i += 2;
        const result = this.parseCSI(data, i);
        i = result;
      } else {
        // Regular character — write to grid
        const ch = String.fromCharCode(data[i]);
        this.writeChar(ch);
        i++;
      }
    }
  }

  /** Parse a CSI sequence starting after ESC[. Returns new index. */
  private parseCSI(data: Uint8Array, start: number): number {
    // Collect parameter bytes (digits and ';')
    let i = start;
    let paramStr = "";
    while (i < data.length) {
      const b = data[i];
      if ((b >= 0x30 && b <= 0x39) || b === 0x3b) {
        // digit or ';'
        paramStr += String.fromCharCode(b);
        i++;
      } else {
        break;
      }
    }

    if (i >= data.length) return i;

    const finalByte = String.fromCharCode(data[i]);
    i++; // consume final byte

    const params = paramStr.length > 0 ? paramStr.split(";").map(Number) : [];

    if (finalByte === "H") {
      // CUP — cursor position (1-based in ANSI)
      const row = (params[0] ?? 1) - 1;
      const col = (params[1] ?? 1) - 1;
      this.cursorRow = row;
      this.cursorCol = col;
    } else if (finalByte === "m") {
      // SGR — set graphic rendition
      this.applySGR(params);
    }

    return i;
  }

  /** Apply SGR parameters. */
  private applySGR(params: number[]): void {
    if (params.length === 0) {
      this.resetStyle();
      return;
    }

    let i = 0;
    while (i < params.length) {
      const p = params[i];
      if (p === SGR_RESET) {
        this.resetStyle();
        i++;
      } else if (p === SGR_BOLD) {
        this.currentBold = true;
        i++;
      } else if (p === SGR_ITALIC) {
        this.currentItalic = true;
        i++;
      } else if (p === SGR_FG_EXTENDED && i + 1 < params.length && params[i + 1] === SGR_RGB_INDICATOR) {
        // 38;2;R;G;B
        if (i + 4 < params.length) {
          this.currentFg = rgbToHex(params[i + 2], params[i + 3], params[i + 4]);
          i += 5;
        } else {
          i++;
        }
      } else if (p === SGR_BG_EXTENDED && i + 1 < params.length && params[i + 1] === SGR_RGB_INDICATOR) {
        // 48;2;R;G;B
        if (i + 4 < params.length) {
          this.currentBg = rgbToHex(params[i + 2], params[i + 3], params[i + 4]);
          i += 5;
        } else {
          i++;
        }
      } else {
        i++;
      }
    }
  }

  private resetStyle(): void {
    this.currentFg = undefined;
    this.currentBg = undefined;
    this.currentBold = false;
    this.currentItalic = false;
  }

  private writeChar(ch: string): void {
    if (this.cursorRow >= 0 && this.cursorRow < this.rows && this.cursorCol >= 0 && this.cursorCol < this.cols) {
      this.grid[this.cursorRow][this.cursorCol] = {
        bg: this.currentBg,
        bold: this.currentBold,
        ch,
        fg: this.currentFg,
        italic: this.currentItalic,
      };
    }
    this.cursorCol++;
  }

  // -----------------------------------------------------------------------
  // Query methods
  // -----------------------------------------------------------------------

  /** Get the cell at (row, col). */
  cellAt(row: number, col: number): ScreenCell | undefined {
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return undefined;
    return this.grid[row][col];
  }

  /** Get the character at (row, col). */
  textAt(row: number, col: number): string | undefined {
    return this.cellAt(row, col)?.ch;
  }

  /** Get the background color at (row, col). */
  bgAt(row: number, col: number): string | undefined {
    return this.cellAt(row, col)?.bg;
  }

  /** Get the foreground color at (row, col). */
  fgAt(row: number, col: number): string | undefined {
    return this.cellAt(row, col)?.fg;
  }

  /** Find the first occurrence of a text string, returning {row, col} or undefined. */
  findText(text: string): { row: number; col: number } | undefined {
    for (let row = 0; row < this.rows; row++) {
      const rowStr = this.getRowText(row);
      const col = rowStr.indexOf(text);
      if (col !== -1) return { col, row };
    }
    return undefined;
  }

  /** Check whether the screen contains the given text anywhere. */
  containsText(text: string): boolean {
    return this.findText(text) !== undefined;
  }

  /** Get the full text content of a row. */
  getRowText(row: number): string {
    if (row < 0 || row >= this.rows) return "";
    return this.grid[row].map((c) => c.ch).join("");
  }

  /** Get all text content as a single string (rows joined by newlines). */
  getTextContent(): string {
    return this.grid.map((row) => row.map((c) => c.ch).join("")).join("\n");
  }

  /** Render the screen as a human-readable string. */
  toString(): string {
    return this.getTextContent();
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const rgbToHex = (r: number, g: number, b: number): string => {
  const rr = r.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
  const gg = g.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
  const bb = b.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
  return `#${rr}${gg}${bb}`;
};
