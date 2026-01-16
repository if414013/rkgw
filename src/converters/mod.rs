// Converters module - format conversion between APIs
//
// This module provides conversion between OpenAI, Anthropic, and Kiro API formats.
// It follows a layered architecture:
// - core: Unified types and shared conversion logic
// - openai_to_kiro: OpenAI → Kiro conversion
// - anthropic_to_kiro: Anthropic → Kiro conversion
// - kiro_to_openai: Kiro → OpenAI conversion
// - kiro_to_anthropic: Kiro → Anthropic conversion

pub mod core;
pub mod openai_to_kiro;
pub mod anthropic_to_kiro;
pub mod kiro_to_openai;
pub mod kiro_to_anthropic;

// Re-export main conversion functions
