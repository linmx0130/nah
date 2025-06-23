/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::types::NahError;

pub fn launch_editor(filename: &str) -> Result<(), NahError> {
  let editor = std::env::var("EDITOR").unwrap_or("vi".to_owned());
  match std::process::Command::new(editor)
    .arg(filename)
    .spawn()
    .unwrap()
    .wait()
  {
    Ok(exit_status) => {
      if exit_status.success() {
        Ok(())
      } else {
        Err(NahError::editor_error(&format!(
          "return value of the editor is {}",
          exit_status.code().unwrap_or(0)
        )))
      }
    }
    Err(e) => Err(NahError::editor_error(&format!("{}", e))),
  }
}
