import hudson.tasks.test.AbstractTestResultAction
import hudson.model.Actionable
import hudson.tasks.junit.CaseResult

pipeline {
    agent none
    parameters {
            choice(name: 'BUMP', choices: ['minor', 'patch', 'major'], description: 'What to bump when releasing') }
    options {
        buildDiscarder(logRotator(numToKeepStr: '50'))
        disableConcurrentBuilds()
    }
    environment {
        GITHUB_TOKEN = credentials('githubrelease')
        TOOL_NAME = 'feenk-releaser'
        MACOS_INTEL_TARGET = 'x86_64-apple-darwin'
        MACOS_M1_TARGET = 'aarch64-apple-darwin'

        WINDOWS_AMD64_SERVER_NAME = 'daffy-duck'
        WINDOWS_AMD64_TARGET = 'x86_64-pc-windows-msvc'
        WINDOWS_ARM64_SERVER_NAME = 'bugs-bunny'
        WINDOWS_ARM64_TARGET = 'aarch64-pc-windows-msvc'

        LINUX_SERVER_NAME = 'mickey-mouse'
        LINUX_AMD64_TARGET = 'x86_64-unknown-linux-gnu'
    }

    stages {
        stage ('Parallel build') {
            parallel {
                stage ('MacOS x86_64') {
                    agent {
                        label "${MACOS_INTEL_TARGET}"
                    }

                    environment {
                        TARGET = "${MACOS_INTEL_TARGET}"
                        PATH = "$HOME/.cargo/bin:/usr/local/bin/:$PATH"
                    }

                    steps {
                        sh 'git clean -fdx'
                        sh "cargo build --bin ${TOOL_NAME} --release"

                        sh "mv target/release/${TOOL_NAME} ${TOOL_NAME}-${TARGET}"

                        stash includes: "${TOOL_NAME}-${TARGET}", name: "${TARGET}"
                    }
                }
                stage ('MacOS M1') {
                    agent {
                        label "${MACOS_M1_TARGET}"
                    }

                    environment {
                        TARGET = "${MACOS_M1_TARGET}"
                        PATH = "$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
                    }

                    steps {
                        sh 'git clean -fdx'
                        sh "cargo build --bin ${TOOL_NAME} --release"

                        sh "mv target/release/${TOOL_NAME} ${TOOL_NAME}-${TARGET}"

                        stash includes: "${TOOL_NAME}-${TARGET}", name: "${TARGET}"
                    }
                }
                stage ('Linux x86_64') {
                    agent {
                        label "${LINUX_AMD64_TARGET}-${LINUX_SERVER_NAME}"
                    }
                    environment {
                        TARGET = "${LINUX_AMD64_TARGET}"
                        PATH = "$HOME/.cargo/bin:$PATH"
                    }

                    steps {
                        sh 'git clean -fdx'
                        sh "cargo build --bin ${TOOL_NAME} --release"

                        sh "mv target/release/${TOOL_NAME} ${TOOL_NAME}-${TARGET}"

                        stash includes: "${TOOL_NAME}-${TARGET}", name: "${TARGET}"
                    }
                }
                stage ('Windows x86_64') {
                    agent {
                        label "${WINDOWS_AMD64_TARGET}-${WINDOWS_AMD64_SERVER_NAME}"
                    }

                    environment {
                        TARGET = "${WINDOWS_AMD64_TARGET}"
                        CARGO_HOME = "C:\\.cargo"
                        CARGO_PATH = "${CARGO_HOME}\\bin"
                        PATH = "${CARGO_PATH};$PATH"
                    }

                    steps {
                        powershell 'git clean -fdx'

                        powershell "cargo build --bin ${TOOL_NAME} --release"
                        powershell "Move-Item -Path target/release/${TOOL_NAME}.exe -Destination ${TOOL_NAME}-${TARGET}.exe"
                        stash includes: "${TOOL_NAME}-${TARGET}.exe", name: "${TARGET}"
                    }
                }
                stage ('Windows arm64') {
                    agent {
                        label "${WINDOWS_ARM64_TARGET}-${WINDOWS_ARM64_SERVER_NAME}"
                    }

                    environment {
                        TARGET = "${WINDOWS_ARM64_TARGET}"
                        CARGO_HOME = "C:\\.cargo"
                        CARGO_PATH = "${CARGO_HOME}\\bin"
                        PATH = "${CARGO_PATH};$PATH"
                    }

                    steps {
                        powershell 'git clean -fdx'

                        powershell "cargo build --bin ${TOOL_NAME} --release"
                        powershell "Move-Item -Path target/release/${TOOL_NAME}.exe -Destination ${TOOL_NAME}-${TARGET}.exe"
                        stash includes: "${TOOL_NAME}-${TARGET}.exe", name: "${TARGET}"
                    }
                }
            }
        }

        stage ('Deployment') {
            agent {
                label "${MACOS_M1_TARGET}"
            }
            environment {
                PATH = "$HOME/.cargo/bin:$PATH"
                TARGET = "${MACOS_M1_TARGET}"
            }
            when {
                expression {
                    (currentBuild.result == null || currentBuild.result == 'SUCCESS') && env.BRANCH_NAME.toString().equals('main')
                }
            }
            steps {
                unstash "${LINUX_AMD64_TARGET}"
                unstash "${MACOS_INTEL_TARGET}"
                unstash "${MACOS_M1_TARGET}"
                unstash "${WINDOWS_AMD64_TARGET}"
                unstash "${WINDOWS_ARM64_TARGET}"

                sh """
                cargo run --bin feenk-releaser --release -- \
                    --owner feenkcom \
                    --repo releaser-rs \
                    --token GITHUB_TOKEN \
                    release \
                    --bump ${params.BUMP} \
                    --auto-accept \
                    --assets \
                        ${TOOL_NAME}-${LINUX_AMD64_TARGET} \
                        ${TOOL_NAME}-${MACOS_INTEL_TARGET} \
                        ${TOOL_NAME}-${MACOS_M1_TARGET} \
                        ${TOOL_NAME}-${WINDOWS_AMD64_TARGET}.exe \
                        ${TOOL_NAME}-${WINDOWS_ARM64_TARGET}.exe """
            }
        }
    }
}
