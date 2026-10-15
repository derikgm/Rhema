const RULES: [RegExp, string][] = [
  [/\bnueva linea\b/gi, "\n"],
  [/\bnueva línea\b/gi, "\n"],
  [/\bsigno de interrogacion\b/gi, "?"],
  [/\bsigno de interrogación\b/gi, "?"],
  [/\bcoma\b/gi, ","],
  [/\bpunto\b/gi, "."],
];

export function applyVoicePunctuation(text: string): string {
  let out = text;
  for (const [re, rep] of RULES) out = out.replace(re, rep);
  return out;
}
