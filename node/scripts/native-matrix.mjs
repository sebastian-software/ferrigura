import { nativeMatrix } from 'ferramenta-native';

// `--without-macos` leaves the macOS target out: a pull request runs the other platforms, and the
// push to main runs them all (docs/decisions/0005-quality-gates.md).
const matrix = nativeMatrix();
if (process.argv.includes('--without-macos')) {
  matrix.include = matrix.include.filter((entry) => entry.runner !== 'macos-latest');
}
console.log(JSON.stringify(matrix));
