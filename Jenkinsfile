import hudson.tasks.test.AbstractTestResultAction
import hudson.model.Actionable
import hudson.tasks.junit.CaseResult

pipeline {
    agent none
    options {
        buildDiscarder(logRotator(numToKeepStr: '50'))
        disableConcurrentBuilds()
    }
    environment {
        GITHUB_TOKEN = credentials('githubrelease')
        AWSIP = 'ec2-18-197-145-81.eu-central-1.compute.amazonaws.com'

        TOOL_NAME = 'feenk-releaser'
        MACOS_INTEL_TARGET = 'x86_64-apple-darwin'
        MACOS_M1_TARGET = 'aarch64-apple-darwin'
        WINDOWS_AMD64_TARGET = 'x86_64-pc-windows-msvc'
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

                        sh "ditto -c -k --sequesterRsrc target/release/${TOOL_NAME} ${TOOL_NAME}-${TARGET}.zip"

                        stash includes: "${TOOL_NAME}-${TARGET}.zip", name: "${TARGET}"
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

                        sh "ditto -c -k --sequesterRsrc target/release/${TOOL_NAME} ${TOOL_NAME}-${TARGET}.zip"

                        stash includes: "${TOOL_NAME}-${TARGET}.zip", name: "${TARGET}"
                    }
                }
                stage ('Linux x86_64') {
                    agent {
                        label "${LINUX_AMD64_TARGET}"
                    }
                    environment {
                        TARGET = "${LINUX_AMD64_TARGET}"
                        PATH = "$HOME/.cargo/bin:$PATH"
                    }

                    steps {
                        sh 'git clean -fdx'
                        sh "cargo build --bin ${TOOL_NAME} --release"

                        sh """
                            cd target/release/
                            zip -r ${TOOL_NAME}-${TARGET}.zip ${TOOL_NAME}
                            """

                        sh 'mv target/release/${TOOL_NAME}-${TARGET}.zip ./${TOOL_NAME}-${TARGET}.zip'

                        stash includes: "${TOOL_NAME}-${TARGET}.zip", name: "${TARGET}"
                    }
                }
                stage ('Windows x86_64') {
                    agent {
                        label "${WINDOWS_AMD64_TARGET}"
                    }

                    environment {
                        TARGET = "${WINDOWS_AMD64_TARGET}"
                        LLVM_HOME = 'C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\BuildTools\\VC\\Tools\\Llvm\\x64'
                        LIBCLANG_PATH = "${LLVM_HOME}\\bin"
                        CMAKE_PATH = 'C:\\Program Files\\CMake\\bin'
                        MSBUILD_PATH = 'C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\BuildTools\\MSBuild\\Current\\Bin'
                        CARGO_PATH = "${homedrive}${homepath}\\.cargo\\bin"
                        PATH = "${CARGO_PATH};${LIBCLANG_PATH};${MSBUILD_PATH};${CMAKE_PATH};$PATH"
                    }

                    steps {
                        powershell 'git clean -fdx'

                        powershell "cargo build --bin ${TOOL_NAME} --release"

                        powershell "Compress-Archive -Path target/release/${TOOL_NAME}.exe -DestinationPath ${TOOL_NAME}-${TARGET}.zip"
                        stash includes: "${TOOL_NAME}-${TARGET}.zip", name: "${TARGET}"
                    }
                }
            }
        }

        stage ('Deployment') {
            agent {
                label "unix"
            }
            environment {
                PATH = "$HOME/.cargo/bin:$PATH"
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

                sh """
                cargo run --bin feenk-releaser --release -- \
                    --owner feenkcom \
                    --repo releaser-rs \
                    --token GITHUB_TOKEN \
                    --bump-minor \
                    --auto-accept \
                    --assets \
                        ${TOOL_NAME}-${LINUX_AMD64_TARGET}.zip \
                        ${TOOL_NAME}-${MACOS_INTEL_TARGET}.app.zip \
                        ${TOOL_NAME}-${MACOS_M1_TARGET}.app.zip \
                        ${TOOL_NAME}-${WINDOWS_AMD64_TARGET}.app.zip """
            }
        }
    }
}
