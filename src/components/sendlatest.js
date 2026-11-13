// One send at a time, and always the latest. While a send is out, newer calls only mark "again";
// when it finishes, the loop sends once more with whatever is newest then, so the stale choices in
// between are skipped. `send` gets a check, `newer()`, that says whether newer choices are already
// waiting (it may then skip work for the old ones).
//
// A failed send never drops a newer choice: if one is waiting, it is still sent. The loop stops on
// a failure only when nothing newer is waiting, and hands that failure to `failed`.
export function sendLatest(send, failed = () => {}) {
  let sending = false
  let again = false
  const newer = () => again
  return async function run() {
    if (sending) {
      again = true
      return
    }
    sending = true
    try {
      do {
        again = false
        try {
          await send(newer)
        } catch (reason) {
          if (!again) failed(reason)
        }
      } while (again)
    } finally {
      sending = false
    }
  }
}
