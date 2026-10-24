import org.jetbrains.intellij.platform.gradle.TestFrameworkType

repositories {
    mavenCentral()

    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        // Conserva aquí tu dependencia actual de RustRover/IntelliJ.
        // Ejemplo: rustRover("2026.2")

        // Necesario para BasePlatformTestCase, myFixture y las fixtures PSI.
        testFramework(TestFrameworkType.Platform)
    }

    // BasePlatformTestCase usa la infraestructura histórica de JUnit 3/4.
    testImplementation("junit:junit:4.13.2")

    // Workaround recomendado por JetBrains para una dependencia ausente
    // en algunas variantes del test framework.
    testImplementation("org.opentest4j:opentest4j:1.3.0")
}
