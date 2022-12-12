(list the supported third party crates)
(the usage with some of the crates are explained in this section, select the corresponding page on the left.)

# Policy for stableness

(For third-party crate that already has a stable version (such as serde), we will make our support on those crates stable. That is we will make sure the support for the existing stable crates will not be affected without bumping the major version)
(For unstable third-party crate, we will make our support on those crates unstable. That means 1. the default version of the third-party crate can changed (with only a minor version bump), 2. the third-party crate will not be included in the default features. But, we still promise not to remove the support of those crates without a major version bump, and the support for a specific version will be represented by a specific feature tag)
