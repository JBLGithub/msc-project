use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_timestamp() 
    -> Result<u64, String> 
{
    let now = SystemTime::now();
    match now.duration_since(UNIX_EPOCH) {
        Ok(unix_now) => {
            Ok(unix_now.as_secs() * 1_000_000 + unix_now.subsec_micros() as u64)
        }
        Err(err) => {
            Err(format!("log_metric(): failed to get duration since UNIX_EPOCH: {}", err))
        }
    }
}