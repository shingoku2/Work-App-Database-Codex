import type { Config } from "tailwindcss";

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        border: "hsl(215 20% 18%)",
        background: "hsl(222 24% 8%)",
        foreground: "hsl(210 40% 96%)",
        muted: "hsl(218 18% 14%)",
        "muted-foreground": "hsl(215 16% 68%)",
        card: "hsl(220 21% 11%)",
        primary: "hsl(199 89% 48%)",
        accent: "hsl(164 78% 42%)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
} satisfies Config;
