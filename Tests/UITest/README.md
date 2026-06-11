⬆️ [hyperv-operation](../../skills/hyperv-operation/SKILL.md) · ⬇️ [Environment](environment/hyper-v-vm-ee-test.md) · [Overlay input and IME](scenarios/overlay-input-ime.md) · [Card preview navigation](scenarios/card-preview-navigation.md) · [Chinese to English](scenarios/chinese-to-english.md)

# EasyEnglish UI Test Specifications

This folder contains markdown-only UI automation specifications. Rust unit and
integration tests live separately in `Tests/UnitTest/`.

## Scope

UI tests cover Windows desktop behavior that cannot be validated by pure Rust
unit tests:

- flyout wake and focus acquisition;
- caret visibility and keyboard input;
- Chinese IME composition and commit behavior;
- card preview keyboard/mouse activation;
- Chinese to English two-level preview navigation;
- Bing fallback row visibility and activation.

## Required execution target

Run UI automation on Windows inside the dedicated Hyper-V VM named
`vm-ee-test`. The host desktop is not the default target.

Read [the `hyperv-operation` skill](../../skills/hyperv-operation/SKILL.md)
before creating, starting, repairing, or using the VM. See
[the environment specification](environment/hyper-v-vm-ee-test.md) for the
dedicated VM contract.
