const flow = 'flowchart TD\n A[Draft] --> B{Review}\n B -->|Changes| A\n B --> C[Publish]\n';
export const cases = [
  { name: 'flow-default', text: flow },
  { name: 'flow-compat', text: flow, compat: true },
  { name: 'gantt', text: 'gantt\n dateFormat HH:mm\n section Work\n Task :a, 00:00, 30d\n' },
  {
    name: 'venn',
    text: 'venn-beta\n set A\n set B\n set C\n set D\n union A, B\n union C, D\n union A, B, C\n',
  },
  {
    name: 'architecture',
    text: '---\nconfig:\n  architecture:\n    randomize: true\n---\narchitecture-beta\n group g1(cloud)[Left]\n group g2(cloud)[Right]\n service a(server)[A] in g1\n service b(database)[B] in g1\n service c(disk)[C] in g2\n service d(internet)[D]\n a:R -- L:c\n b:T -- B:a\n c:B -- T:d\n d:L -[Link]- R:b\n',
  },
  { name: 'font', text: 'flowchart LR\n A[Wide WWW narrow iii]\n', font: true },
];
