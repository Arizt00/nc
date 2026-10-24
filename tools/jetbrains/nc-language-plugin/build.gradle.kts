import org.jetbrains.intellij.platform.gradle.TestFrameworkType

import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    kotlin("jvm") version "2.3.0"
    id("org.jetbrains.intellij.platform")
}

group = "dev.nc"
version = "1.0.0.21v"

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.opentest4j:opentest4j:1.3.0")

    implementation("com.fasterxml.jackson.core:jackson-databind:2.17.1")
    testImplementation(kotlin("test"))

    intellijPlatform {
        testFramework(TestFrameworkType.Platform)

        intellijIdea("2026.1")
        bundledPlugin("com.intellij.java")
    }
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

kotlin {
    jvmToolchain(21)
}

tasks.withType<KotlinCompile>().configureEach {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

intellijPlatform {
    pluginConfiguration {
        version = project.version.toString()
        description = """
            nC JetBrains integration.
            Semantic authority remains in ncc.
            License: MIT OR Apache-2.0.
        """.trimIndent()
        changeNotes = "1.0.0.21v: canonical compact syntax, extension-aware colors, raw pointer aliases, backend prelude awareness and permissive return ABI."
        ideaVersion {
            sinceBuild = "242"
untilBuild = "262.*"
        }
    }
}
