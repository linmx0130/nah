use std::io::Write;

pub fn ask_for_user_confirmation(msg: &str, msg_on_cancel: &str) -> bool {
  print!("{}", msg);
  let _ = std::io::stdout().flush();
  let mut buf = String::new();
  let _ = std::io::stdin().read_line(&mut buf);
  let result = buf.trim();
  if !(result == "Y" || result == "y") {
    println!("{}", msg_on_cancel);
    false
  } else {
    true
  }
}
