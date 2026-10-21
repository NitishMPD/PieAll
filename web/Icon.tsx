const paths: Record<string, string> = {
  apps: "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
  convert: "M4 8h14m-4-4 4 4-4 4M20 16H6m4-4-4 4 4 4",
  crop: "M7 3v14h14M3 7h14v14",
  resize: "M4 9V4h5M4 4l6 6m10 5v5h-5m5 0-6-6",
  frame: "M4 4h16v16H4zM8 8h8v8H8z",
  compress: "m4 4 6 6M5 10h5V5m10 15-6-6m-1 5v-5h5",
  more: "M5 12h.01M12 12h.01M19 12h.01",
  images: "M4 4h16v16H4zM4 16l5-5 4 4 3-3 4 4M15 8h.01",
  files: "M4 6h6l2 2h8v12H4z",
  text: "M4 5h16M12 5v15M8 20h8",
  archives: "M5 4h14v16H5zM12 4v3m0 3v3m0 3v1",
  window: "M3 5h18v14H3zM3 9h18",
  search: "M16 16l5 5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
  open: "M12 16V3m-5 5 5-5 5 5M4 14v6h16v-6",
  clipboard: "M9 4H5v17h14V4h-4M9 2h6v5H9z",
  close: "m6 6 12 12M6 18 18 6",
  back: "m14 5-7 7 7 7M7 12h14",
  arrow: "M4 12h16m-6-6 6 6-6 6",
  check: "m5 12 4 4L19 6",
  copy: "M8 8h12v12H8zM4 16V4h12",
  update: "M20 10a8 8 0 1 0-1 7M20 3v7h-7",
  ring: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
  alert:
    "M12 8v5m0 3h.01M10.3 3.9 2.6 17.2A2 2 0 0 0 4.3 20h15.4a2 2 0 0 0 1.7-2.8L13.7 3.9a2 2 0 0 0-3.4 0",
  rotate: "M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5",
  "rotate-left": "M4 12a8 8 0 1 0 2.3-5.7M4 4v5h5",
  flip: "M12 3v18M8 7 3 12l5 5V7m8 0 5 5-5 5V7",
  "flip-v": "M3 12h18M7 8l5-5 5 5H7m0 8 5 5 5-5H7",
  clean:
    "M4 20 14 10m-4-4 8 8M15 3l1.5 3L20 7l-3.5 1.5L15 12l-1.5-3.5L10 7l3.5-1z",
  pdf: "M6 3h9l4 4v14H6zM14 3v5h5M9 13h6m-6 4h4",
  grayscale: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0M12 3v18",
  grid: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
  palette:
    "M12 3a9 9 0 0 0 0 18c1.1 0 1.5-.8 1.5-1.5 0-1.5 1-2.5 2.5-2.5H18a3 3 0 0 0 3-3c0-6-4-11-9-11M7.5 11h.01M10 7h.01M15 7.5h.01",
  extract: "M5 4h14v16H5zM12 8v8m-3-3 3 3 3-3",
  duplicate: "M8 8h12v12H8zM4 16V4h12M14 11v6m-3-3h6",
  terminal: "M3 5h18v14H3zM7 10l3 2-3 2m5 1h4",
  hash: "M9 3 7 21M17 3l-2 18M4 8h17M3 16h17",
  case: "M3 18 7.5 6 12 18M4.7 14h5.6M15 12.5a2.5 2.5 0 1 1 0 5h-.5V12.5zM14.5 18v-8",
  trim: "M3 6h18M7 12h10M3 18h18M3 9v6m18-6v6",
  sort: "M4 6h10M4 12h7M4 18h4M17 4v16m-3-3 3 3 3-3",
  json: "M8 4H7a2 2 0 0 0-2 2v4l-2 2 2 2v4a2 2 0 0 0 2 2h1m8-16h1a2 2 0 0 1 2 2v4l2 2-2 2v4a2 2 0 0 1-2 2h-1",
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  code: "m8 7-5 5 5 5m8-10 5 5-5 5M14 4l-4 16",
  info: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0M12 11v5m0-8h.01",
  center: "M3 5h18v14H3zM9 9h6v6H9z",
  pin: "M9 3h6l-1 6 4 3v2H6v-2l4-3zM12 14v7",
  up: "m6 15 6-6 6 6",
  down: "m6 9 6 6 6-6",
  plus: "M12 5v14M5 12h14",
  keyboard: "M3 6h18v12H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10",
  sliders: "M4 7h9m4 0h3M4 17h3m4 0h9M15 5v4M9 15v4",
  home: "M4 11 12 4l8 7v9H4z",
};
export function Icon({ name, size = 22 }: { name: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name.toLowerCase()] ?? paths.files} />
    </svg>
  );
}
