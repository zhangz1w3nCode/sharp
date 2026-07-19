/* =========================================================================
   base-resource/theme/tokens.ts  —  design tokens for the Tauri/React app
   mirror of tokens.css (single source: theme/DESIGN.md).
   Use: import { tokens } from "@/base-resource/theme/tokens"
   ========================================================================= */

export const tokens = {
  color: {
    primary: "#cc785c", primaryActive: "#a9583e", primaryDisabled: "#e6dfd8",
    ink: "#141413", body: "#3d3d3a", bodyStrong: "#252523", muted: "#6c6a64", mutedSoft: "#8e8b82",
    canvas: "#faf9f5", surfaceSoft: "#f5f0e8", surfaceCard: "#efe9de", surfaceCreamStrong: "#e8e0d2",
    surfaceDark: "#181715", surfaceDarkElevated: "#252320", surfaceDarkSoft: "#1f1e1b",
    onPrimary: "#ffffff", onDark: "#faf9f5", onDarkSoft: "#a09d96",
    accentTeal: "#5db8a6", accentAmber: "#e8a55a",
    success: "#5db872", warning: "#d4a017", error: "#c64545",
    hairline: "#e6dfd8", hairlineSoft: "#ebe6df", hairlineDark: "#2f2c28",
  } as const,

  font: {
    serif: "'Cormorant Garamond','Tiempos Headline','Songti SC','Source Han Serif SC',Georgia,serif",
    sans: "'StyreneB','Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,system-ui,sans-serif",
    mono: "'JetBrains Mono',ui-monospace,Menlo,Consolas,monospace",
  } as const,

  radius: { xs: 4, sm: 6, md: 8, lg: 12, xl: 16, pill: 9999 } as const,

  spacing: { xxs: 4, xs: 8, sm: 12, md: 16, lg: 24, xl: 32, xxl: 48, section: 96 } as const,

  /* type scale (ref DESIGN.md typography) */
  type: {
    displayXl: { size: 64, weight: 400, lineHeight: 1.05, tracking: -1.5 },
    displayLg: { size: 48, weight: 400, lineHeight: 1.1, tracking: -1 },
    displayMd: { size: 36, weight: 400, lineHeight: 1.15, tracking: -0.5 },
    displaySm: { size: 28, weight: 400, lineHeight: 1.2, tracking: -0.3 },
    titleLg: { size: 22, weight: 500, lineHeight: 1.3, tracking: 0 },
    titleMd: { size: 18, weight: 500, lineHeight: 1.4, tracking: 0 },
    titleSm: { size: 16, weight: 500, lineHeight: 1.4, tracking: 0 },
    bodyMd: { size: 16, weight: 400, lineHeight: 1.55, tracking: 0 },
    bodySm: { size: 14, weight: 400, lineHeight: 1.55, tracking: 0 },
    caption: { size: 13, weight: 500, lineHeight: 1.4, tracking: 0 },
    captionUppercase: { size: 12, weight: 500, lineHeight: 1.4, tracking: 1.5 },
    code: { size: 14, weight: 400, lineHeight: 1.6, tracking: 0 },
    button: { size: 14, weight: 500, lineHeight: 1, tracking: 0 },
    navLink: { size: 14, weight: 500, lineHeight: 1.4, tracking: 0 },
  } as const,

  shadow: {
    window: "0 24px 70px rgba(20,20,19,.18), 0 0 0 1px #e6dfd8",
    float: "0 8px 24px rgba(20,20,19,.10)",
    modal: "0 18px 50px rgba(20,20,19,.12)",
  } as const,

  ring: { coral: "0 0 0 3px rgba(204,120,92,.15)" } as const,
} as const;

export type Tokens = typeof tokens;
