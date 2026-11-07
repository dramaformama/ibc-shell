/** Own simple password strength meter: 0 (empty) … 4 (strong). Length-led, no dictionaries. */
export type Strength = 0 | 1 | 2 | 3 | 4;

export function passwordStrength(pw: string): Strength {
  if (!pw) return 0;
  const classes = [/[a-zа-я]/, /[A-ZА-Я]/, /\d/, /[^\p{L}\d]/u].filter((r) => r.test(pw)).length;
  const unique = new Set(pw).size;
  let bits = Math.min(unique, pw.length) * (1 + classes * 0.9);
  if (/^(.)\1+$/.test(pw) || /^(password|qwerty|12345)/i.test(pw)) bits /= 3;
  if (pw.length < 8) return bits > 14 ? 2 : 1;
  if (bits < 28) return 1;
  if (bits < 50) return 2;
  if (bits < 75) return 3;
  return 4;
}

/** Minimum length accepted for a master password. */
export const MIN_PASSWORD = 8;
