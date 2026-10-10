export function isInvalidRecoveryEmail(input: HTMLInputElement) {
  return input.value.length > 0 && !input.validity.valid;
}
export function recoveryPasswordValidation(password: string, confirmation: string) {
  const matches = password === confirmation;
  return { invalid: password.length < 8 || !matches, mismatch: confirmation.length > 0 && !matches };
}
