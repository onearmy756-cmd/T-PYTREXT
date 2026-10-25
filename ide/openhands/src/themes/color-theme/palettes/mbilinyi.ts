// MBILINYI "Obsidian Ember" palette — kaboni nyeusi yenye joto la ember.
//
// Identity: ukungu mweusi wa obsidian (hue 15°) ukichochewa na ember ya
// machungwa-nyekundu. Sio graphite-bluu, sio neutral grey, sio teal. Ni
// mwonekano wa moto ndani ya giza — joto na mkali, lakini bado mtulivu.
// WCAG-readable contrast inahifadhiwa.

// HSL channel strings (H S% L%) so Tailwind utilities resolve via hsl(var(--heroui-*)).
// prettier-ignore
const MBILINYI_HSL = {
  50:  "25.71 100.00% 95.88%", // #FFF3EA
  100: "24.00 100.00% 91.18%",  // #FFE4D2
  200: "22.25 100.00% 82.55%",  // #FFC7A6
  300: "22.56 89.26% 70.78%",   // #F7A472
  400: "22.06 71.43% 57.45%",   // #E07E45
  500: "20.57 61.40% 44.71%",   // #B85C2C
  600: "20.00 63.43% 34.31%",   // #8F4520
  700: "18.99 60.31% 25.69%",   // #69331A
  800: "18.00 54.35% 18.04%",   // #472415
  850: "18.00 52.63% 14.90%",   // #3A1E12
  900: "15.56 49.09% 10.78%",   // #29150E
  925: "15.00 50.00% 7.84%",    // #1E0F0A
  950: "15.00 46.15% 5.10%",    // #130A07
  975: "17.14 53.85% 2.55%",    // #0A0503
};

export const MBILINYI_SCALE = {
  "--cool-grey-50": "#FFF3EA",
  "--cool-grey-100": "#FFE4D2",
  "--cool-grey-200": "#FFC7A6",
  "--cool-grey-300": "#F7A472",
  "--cool-grey-400": "#E07E45",
  "--cool-grey-500": "#B85C2C",
  "--cool-grey-600": "#8F4520",
  "--cool-grey-700": "#69331A",
  "--cool-grey-800": "#472415",
  "--cool-grey-900": "#29150E",
  "--cool-grey-925": "#1E0F0A",
  "--cool-grey-950": "#130A07",
  "--cool-grey-975": "#0A0503",
};

export const MBILINYI_HEROUI = {
  "--heroui-background": MBILINYI_HSL[950],
  "--heroui-background-foreground": MBILINYI_HSL[50],
  "--heroui-foreground-50": MBILINYI_HSL[975],
  "--heroui-foreground-100": MBILINYI_HSL[950],
  "--heroui-foreground-200": MBILINYI_HSL[900],
  "--heroui-foreground-300": MBILINYI_HSL[850],
  "--heroui-foreground-400": MBILINYI_HSL[800],
  "--heroui-foreground-500": MBILINYI_HSL[700],
  "--heroui-foreground-600": MBILINYI_HSL[600],
  "--heroui-foreground-700": MBILINYI_HSL[500],
  "--heroui-foreground-800": MBILINYI_HSL[400],
  "--heroui-foreground-900": MBILINYI_HSL[300],
  "--heroui-foreground": MBILINYI_HSL[300],
  "--heroui-content1": MBILINYI_HSL[900],
  "--heroui-content1-foreground": MBILINYI_HSL[100],
  "--heroui-content2": MBILINYI_HSL[850],
  "--heroui-content2-foreground": MBILINYI_HSL[200],
  "--heroui-content3": MBILINYI_HSL[800],
  "--heroui-content3-foreground": MBILINYI_HSL[300],
  "--heroui-content4": MBILINYI_HSL[700],
  "--heroui-content4-foreground": MBILINYI_HSL[400],
  "--heroui-default-50": MBILINYI_HSL[975],
  "--heroui-default-100": MBILINYI_HSL[950],
  "--heroui-default-200": MBILINYI_HSL[900],
  "--heroui-default-300": MBILINYI_HSL[850],
  "--heroui-default-400": MBILINYI_HSL[800],
  "--heroui-default-500": MBILINYI_HSL[700],
  "--heroui-default-600": MBILINYI_HSL[600],
  "--heroui-default-700": MBILINYI_HSL[500],
  "--heroui-default-800": MBILINYI_HSL[400],
  "--heroui-default-900": MBILINYI_HSL[300],
  "--heroui-default-foreground": MBILINYI_HSL[50],
  "--heroui-default": MBILINYI_HSL[800],
};
