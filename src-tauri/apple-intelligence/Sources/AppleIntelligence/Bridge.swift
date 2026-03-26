import Foundation
import SwiftRs

#if canImport(FoundationModels)
import FoundationModels
#endif

// MARK: - JSON helpers

private func jsonOk(_ text: String) -> String {
    let obj: [String: String] = ["ok": text]
    guard let data = try? JSONSerialization.data(withJSONObject: obj),
          let json = String(data: data, encoding: .utf8) else {
        return #"{"ok":""}"#
    }
    return json
}

private func jsonErr(_ error: String, _ message: String) -> String {
    let obj: [String: String] = ["error": error, "message": message]
    guard let data = try? JSONSerialization.data(withJSONObject: obj),
          let json = String(data: data, encoding: .utf8) else {
        return #"{"error":"\#(error)","message":""}"#
    }
    return json
}

// Thread-safe container for passing results across Task boundaries in Swift 6.
private final class ResultBox: @unchecked Sendable {
    var value: String
    init(_ value: String) { self.value = value }
}

// MARK: - Availability check

@_cdecl("ai_check_availability")
public func aiCheckAvailability() -> Bool {
    #if canImport(FoundationModels)
    guard #available(macOS 26.0, *) else {
        return false
    }
    return SystemLanguageModel.default.availability == .available
    #else
    return false
    #endif
}

// MARK: - Text generation

/// Synchronous wrapper around Foundation Models async API.
/// Uses DispatchSemaphore + Task to bridge async/sync boundary.
/// Must be called from a non-main thread (Tauri commands satisfy this).
@_cdecl("ai_generate")
public func aiGenerate(system: SRString, user: SRString) -> SRString {
    #if canImport(FoundationModels)
    guard #available(macOS 26.0, *) else {
        return SRString(jsonErr("unsupported_os", "macOS 26.0 or later is required"))
    }

    let systemPrompt = system.toString()
    let userPrompt = user.toString()

    let box = ResultBox(jsonErr("unknown", "Generation did not complete"))
    let semaphore = DispatchSemaphore(value: 0)

    Task {
        do {
            let session = LanguageModelSession(instructions: systemPrompt)
            let response = try await session.respond(to: userPrompt)
            box.value = jsonOk(response.content)
        } catch let error as LanguageModelSession.GenerationError {
            switch error {
            case .guardrailViolation:
                box.value = jsonErr("guardrail_violation", "Content flagged by safety guardrails")
            case .exceededContextWindowSize:
                box.value = jsonErr("context_exceeded", "Context window size exceeded")
            case .assetsUnavailable:
                box.value = jsonErr("assets_unavailable", "Model assets are not available")
            case .unsupportedLanguageOrLocale:
                box.value = jsonErr("unsupported_language", "Language or locale is not supported")
            case .rateLimited:
                box.value = jsonErr("rate_limited", "Rate limit exceeded")
            case .concurrentRequests:
                box.value = jsonErr("concurrent_requests", "Too many concurrent requests")
            case .decodingFailure:
                box.value = jsonErr("decoding_failure", "Model output could not be decoded")
            case .refusal:
                box.value = jsonErr("refusal", "Model refused to generate content")
            case .unsupportedGuide:
                box.value = jsonErr("unsupported_guide", "Requested generation guide is not supported")
            @unknown default:
                box.value = jsonErr("generation_error", error.localizedDescription)
            }
        } catch {
            box.value = jsonErr("unknown", error.localizedDescription)
        }
        semaphore.signal()
    }

    semaphore.wait()
    return SRString(box.value)
    #else
    return SRString(jsonErr("unsupported_sdk", "FoundationModels framework not available"))
    #endif
}
