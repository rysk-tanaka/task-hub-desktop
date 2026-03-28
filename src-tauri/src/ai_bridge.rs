//! Apple Foundation Models ブリッジ
//!
//! macOS 26+ の on-device AI テキスト生成を提供する。
//! 非 macOS 環境ではスタブが使用され、`is_available()` は常に `false` を返す。
//!
//! # Safety
//! `generate()` は Swift 側で `DispatchSemaphore.wait()` によりブロックするため、
//! メインスレッドから呼び出すとデッドロックする可能性がある。Tauri のワーカースレッドから呼ぶこと。
//! `is_available()` は非ブロッキングであり、任意のスレッドから安全に呼び出せる。

#[cfg(target_os = "macos")]
mod platform {
    use serde::Deserialize;
    use swift_rs::{swift, Bool, SRString};

    swift!(fn ai_check_availability() -> Bool);
    swift!(fn ai_generate(system: &SRString, user: &SRString) -> SRString);

    #[derive(Deserialize)]
    struct AiOk {
        ok: String,
    }

    #[derive(Deserialize)]
    struct AiErr {
        error: String,
        message: String,
    }

    pub fn is_available() -> bool {
        // Safety: FFI call with no mutable state; safe to call from any thread.
        unsafe { ai_check_availability() }
    }

    pub fn generate(system: &str, user: &str) -> Result<String, String> {
        let system = SRString::from(system);
        let user = SRString::from(user);

        // Safety: FFI call that blocks via DispatchSemaphore internally.
        // Must not be called from the main thread.
        let json = unsafe { ai_generate(&system, &user) };
        let json_str: &str = json.as_str();

        if let Ok(ok) = serde_json::from_str::<AiOk>(json_str) {
            return Ok(ok.ok);
        }
        if let Ok(err) = serde_json::from_str::<AiErr>(json_str) {
            return Err(format!("{}: {}", err.error, err.message));
        }
        Err(format!("unexpected response from AI bridge: {json_str}"))
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn is_available() -> bool {
        false
    }

    pub fn generate(_system: &str, _user: &str) -> Result<String, String> {
        Err("Apple Intelligence is only available on macOS".to_string())
    }
}

pub use platform::is_available;
pub use platform::generate;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_available_does_not_panic() {
        // FFI 呼び出しがクラッシュせず bool を返すことを確認
        let result = is_available();
        println!("AI availability: {result}");
    }
}
