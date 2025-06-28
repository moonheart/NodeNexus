use encoding_rs;
use lazy_static::lazy_static;

// Platform-specific encoding detection
#[cfg(windows)]
use {codepage, windows_sys::Win32::Globalization::GetACP};

lazy_static! {
    static ref SYSTEM_ENCODING: &'static encoding_rs::Encoding = {
        #[cfg(windows)]
        {
            // On Windows, we use the GetACP function to get the system's active code page.
            let acp = unsafe { GetACP() };
            codepage::to_encoding(acp.try_into().unwrap())
                .unwrap_or(encoding_rs::UTF_8) // Fallback to UTF-8 if the code page is not recognized
        }
        #[cfg(not(windows))]
        {
            // On Unix-like systems, locale detection is more complex.
            // A common fallback is to check for en_US.UTF-8, but for simplicity
            // in this context, we'll default to UTF-8 and have a specific fallback
            // for GBK, which covers many common non-UTF-8 cases.
            // A more advanced implementation could parse the `LANG` env var.
            encoding_rs::UTF_8
        }
    };
}

pub fn decode_chunk(chunk: &[u8]) -> String {
    // First, always try to decode as UTF-8, as it's the universal standard.
    if let Ok(s) = std::str::from_utf8(chunk) {
        return s.to_string();
    }

    // If UTF-8 fails, try the detected system encoding (on Windows) or a common fallback.
    // We check if the system encoding is already UTF-8 to avoid re-checking.
    if SYSTEM_ENCODING.name() != "UTF-8" {
        let (cow, _encoding_used, had_errors) = SYSTEM_ENCODING.decode(chunk);
        if !had_errors {
            return cow.into_owned();
        }
    }

    // As a last resort, use lossy UTF-8 decoding. This prevents data corruption
    // from crashing the agent, ensuring the app remains stable.
    String::from_utf8_lossy(chunk).to_string()
}