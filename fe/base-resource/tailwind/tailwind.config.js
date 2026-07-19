/* =========================================================================
   base-resource/tailwind/tailwind.config.js  —  shared Tailwind config (off CDN)
   Maps Claude design tokens → Tailwind theme. Import in a real Tailwind build
   (Vite/postcss), replacing the inline `tailwind.config=...` used in prototypes.
   Token values live in component/tokens.css as CSS vars; this just wires names.
   ========================================================================= */
module.exports = {
  content: [
    '../../prototypes-v2/**/*.html',
    '../../app/**/*.{ts,tsx,html}',
  ],
  theme: {
    extend: {
      colors: {
        ink: 'var(--ink)', body: 'var(--body)', 'body-strong': 'var(--body-strong)',
        muted: 'var(--muted)', 'muted-soft': 'var(--muted-soft)',
        canvas: 'var(--canvas)', 'surface-soft': 'var(--surface-soft)',
        card: 'var(--surface-card)', 'cream-strong': 'var(--surface-cream-strong)',
        dark: 'var(--surface-dark)', 'dark-elev': 'var(--surface-dark-elevated)', 'dark-soft': 'var(--surface-dark-soft)',
        'on-dark': 'var(--on-dark)', 'on-dark-soft': 'var(--on-dark-soft)',
        accent: 'var(--primary)', 'accent-active': 'var(--primary-active)',
        teal: 'var(--accent-teal)', amber: 'var(--accent-amber)',
        success: 'var(--success)', warning: 'var(--warning)', error: 'var(--error)',
        hairline: 'var(--hairline)', 'hairline-soft': 'var(--hairline-soft)',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'Roboto', 'system-ui', 'sans-serif'],
        serif: ['"Cormorant Garamond"', 'Songti SC', '"Source Han Serif SC"', 'Georgia', 'serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'Menlo', 'Consolas', 'monospace'],
      },
      borderRadius: { xs: '4px', sm: '6px', md: '8px', lg: '12px', xl: '16px' },
      boxShadow: {
        window: '0 24px 70px rgba(20,20,19,.18), 0 0 0 1px var(--hairline)',
        float: '0 8px 24px rgba(20,20,19,.10)',
        modal: '0 18px 50px rgba(20,20,19,.12)',
      },
    },
  },
  plugins: [],
};
