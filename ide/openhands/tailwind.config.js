/** @type {import('tailwindcss').Config} */
import typography from "@tailwindcss/typography";
export default {
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        modal: {
          background: "#1E0F0A",
          input: "#472415",
          primary: "#FF6B2C",
          secondary: "#8F4520",
          muted: "#B85C2C",
        },
        surface: {
          DEFAULT: "#0A0503",
          card: "#130A07",
          elevated: "#29150E",
          outline: "#1E0F0A",
          background: "#472415",
          divider: "#8F4520",
          button: "#B85C2C",
          text: "#E07E45",
        },
        content: {
          DEFAULT: "#FFE4D2",
          muted: "#B85C2C",
          icon: "#69331A",
        },
        status: {
          "success-bg": "rgba(74, 222, 128, 0.1)",
          "success-border": "rgba(74, 222, 128, 0.4)",
          "success-text": "#86efac",
          "success-badge-bg": "rgba(74, 222, 128, 0.15)",
          "fail-bg": "rgba(240, 113, 107, 0.1)",
          "fail-border": "rgba(240, 113, 107, 0.4)",
          "fail-text": "#fda4af",
          "fail-solid": "#dc2626",
          "fail-solid-hover": "#b91c1c",
        },
        toggle: {
          active: "#FF6B2C",
          "active-bg": "rgba(255, 107, 44, 0.2)",
          "active-border": "rgba(255, 107, 44, 0.5)",
          inactive: "#29150E",
          "inactive-knob": "#B85C2C",
          "inactive-border": "#69331A",
        },
        "muted-overlay": "rgba(10, 5, 3, 0.4)",
        "pill-bg": "rgba(41, 21, 14, 0.3)",
      },
    },
  },
  plugins: [typography],
};
