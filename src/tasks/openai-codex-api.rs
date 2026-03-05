pub(super) fn ask_openai_codex_api(prompt: &str) -> Result<String, String> {
    super::ask_openai_responses(
        prompt,
        "OPENAI_CODEX_MODEL",
        "gpt-5.3-codex",
        "openai codex api",
    )
}
