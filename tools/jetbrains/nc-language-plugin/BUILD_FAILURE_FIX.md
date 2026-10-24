# Corrección del fallo `compileTestKotlin`

El código principal del plugin compila. El fallo ocurre exclusivamente al compilar las pruebas porque el classpath de test no contiene la infraestructura de IntelliJ Platform:

- `com.intellij.testFramework.fixtures.BasePlatformTestCase`
- `myFixture`
- las aserciones heredadas de la clase base

La solución es declarar explícitamente el test framework de la plataforma y JUnit en `build.gradle.kts`.

## Corrección automática

```powershell
$PluginRoot = "C:\Users\arist\Documents\Codex\2026-04-24\proyecto-nc-ncc-objetivo-construir-un\ncc_codex_bundle\codex_bundle\tools\jetbrains\nc-language-plugin"

.\scripts\Fix-NcPluginTestFramework.ps1 `
    -PluginRoot $PluginRoot `
    -Build `
    -RefreshDependencies
```

## Corrección manual

Añade al principio de `build.gradle.kts`:

```kotlin
import org.jetbrains.intellij.platform.gradle.TestFrameworkType
```

Dentro de `dependencies { intellijPlatform { ... } }`:

```kotlin
testFramework(TestFrameworkType.Platform)
```

Dentro de `dependencies { ... }`, pero fuera del bloque `intellijPlatform`:

```kotlin
testImplementation("junit:junit:4.13.2")
testImplementation("org.opentest4j:opentest4j:1.3.0")
```

Después:

```powershell
.\gradlew.bat clean test buildPlugin --refresh-dependencies
```

Los avisos de `NcTextAttributes.kt` no provocan el fallo. Indican que el archivo utiliza el overload antiguo que recibe `TextAttributes` fijos. La migración recomendada es crear las claves con una `TextAttributesKey` de fallback, por ejemplo:

```kotlin
TextAttributesKey.createTextAttributesKey(
    "NC_KEYWORD",
    DefaultLanguageHighlighterColors.KEYWORD
)
```

No conviene reemplazar ese archivo automáticamente sin revisar qué categoría representa cada clave.
