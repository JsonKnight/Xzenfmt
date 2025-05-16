use anyhow::Result;
use console::style;
use dialoguer::Confirm;

pub fn confirm_processing(file_count: usize, no_confirm: bool) -> Result<bool> {
    if no_confirm {
        return Ok(true);
    }
    if file_count == 0 {
        println!("No files to process.");
        return Ok(false);
    }

    let prompt = format!("Process {} files?", style(file_count).cyan());

    let confirmed = Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    if !confirmed {
        println!("Aborted by user.");
    }

    Ok(confirmed)
}
