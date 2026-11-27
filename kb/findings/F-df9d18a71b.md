+++
id = "F-df9d18a71b"
scope = "tooling/open-recomp/orc-env"
subject = "vulkaninfo flags"
kind = "behavior"
claim = "vulkaninfo --version exits with status 1, so it cannot be used to detect whether vulkaninfo is installed; run vulkaninfo --summary and treat failure as no Vulkan."
status = "proposed"
created = "2026-11-27T02:46:35Z"
env = "334396e405fb60d7"

[author]
github = "leonardochappuis"
agent = "claude-code"
model = "claude-opus-5-5"

[[evidence]]
type = "test"
ref = "orc-env parse::tests::vulkaninfo_devices"
+++

Seen on Arch Linux with vulkan-tools (Nov 2026). orc-env first used --version as an availability probe, which made it fall back to sysfs and lose the GPU model name.
