export default {
  '!(docs/**/*)*.{ts,js,html,md,mts}': [
    'eslint --cache --cache-strategy content --fix',
    'prettier --write',
  ],
  '.cspell/*.txt': ['tsx scripts/fixCSpell.ts'],
};
