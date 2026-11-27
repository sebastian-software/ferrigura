const flow = 'flowchart TD\n A[Draft] --> B{Review}\n B -->|Changes| A\n B --> C[Publish]\n';
export const cases = [
  { name: 'flow-default', text: flow },
  { name: 'flow-compat', text: flow, compat: true },
  { name: 'gantt', text: 'gantt\n dateFormat HH:mm\n section Work\n Task :a, 00:00, 30d\n' },
  {
    name: 'mindmap',
    text: 'mindmap\n root((Release))\n  Research\n   Interviews\n   Surveys\n  Build\n   Parser\n   Layout\n   SVG\n  Ship\n',
  },
  { name: 'font', text: 'flowchart LR\n A[Wide WWW narrow iii]\n', font: true },
];
