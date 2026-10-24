# nC JetBrains Navigation Upgrade

Sustitución modular y mantenible de `NcGotoDeclarationHandler` para el plugin de JetBrains de nC.

## Qué incluye

- Handler de extensión completamente stateless.
- Servicio de proyecto reutilizable por navegación, documentación y futuras referencias PSI.
- Caché por `PsiFile`, invalidada al cambiar el archivo.
- Resolución de ámbitos léxicos que ignora llaves dentro de strings y comentarios.
- Resolución de módulos mediante VFS e índices de IntelliJ, sin `File.walkTopDown()`.
- Soporte para `.n`, `.nc`, `.nm`, `.ncp`, `.nb` y `.nbb`.
- Navegación a imports, rutas cualificadas, campos, símbolos locales, símbolos importados y documentación.
- Filtrado de declaraciones privadas entre módulos.
- Soporte de varios destinos cuando dos módulos exportan el mismo nombre.
- Protección contra rutas de documentación que intenten escapar del proyecto.
- Pruebas unitarias y pruebas ligeras de integración.
- Instalador PowerShell con copia de seguridad.

## Dependencias existentes

El paquete está diseñado para integrarse con las clases que ya utiliza tu handler actual:

- `NcLanguage`
- `NcSourceIndex`
- `NcIntellisenseCatalog`

No reemplaza esas clases. Usa sus APIs existentes:

- `tokenAt`
- `tokenStartAt`
- `declarations`
- `typedSymbols`
- `fieldsByType`
- `baseAggregateType`
- `elementAggregateType`
- `cleanToken`
- `lookup`

## Instalación automática

```powershell
$PluginRoot = "C:\Users\arist\Documents\Codex\2026-04-24\proyecto-nc-ncc-objetivo-construir-un\ncc_codex_bundle\codex_bundle\tools\jetbrains\nc-language-plugin"

Set-Location "RUTA_DONDE_EXTRAJISTE_NcNavigationUpgrade_complete"

.\scripts\Install-NcNavigationUpgrade.ps1 `
    -PluginRoot $PluginRoot `
    -Build
```

El script:

1. crea una copia de seguridad;
2. copia las clases Kotlin;
3. registra `NcGotoDeclarationHandler` en `plugin.xml`;
4. copia las pruebas;
5. ejecuta `clean buildPlugin` si se usa `-Build`.

## Instalación manual

Copia los archivos de:

```text
src/main/kotlin/dev/nc/jetbrains
```

a:

```text
<plugin>/src/main/kotlin/dev/nc/jetbrains
```

Añade dentro de:

```xml
<extensions defaultExtensionNs="com.intellij">
```

esta línea:

```xml
<gotoDeclarationHandler implementation="dev.nc.jetbrains.NcGotoDeclarationHandler"/>
```

La clase `NcNavigationService` usa `@Service(Service.Level.PROJECT)`, así que no requiere una segunda entrada en `plugin.xml`.

## Dependencias de pruebas

Con IntelliJ Platform Gradle Plugin 2.x, añade dentro de `dependencies`:

```kotlin
import org.jetbrains.intellij.platform.gradle.TestFrameworkType

dependencies {
    intellijPlatform {
        testFramework(TestFrameworkType.Platform)
    }

    testImplementation("junit:junit:4.13.2")
}
```

## Construcción

```powershell
Set-Location $PluginRoot
.\gradlew.bat clean test buildPlugin
```

Para probar en una instancia aislada:

```powershell
.\gradlew.bat runIde
```

## Arquitectura

```text
NcGotoDeclarationHandler
        ↓
NcNavigationService
        ├── NcFileAnalysisCache
        │       └── NcLexicalScopeMap
        ├── NcProjectModuleResolver
        └── NcDocumentationResolver
```

La extensión no guarda estado, no inicializa índices y no recorre el disco. El trabajo reutilizable vive en el servicio de proyecto.

## Orden de resolución

1. módulo dentro de `use <project:...>`;
2. ruta cualificada `module.symbol`;
3. campo `object.field`;
4. declaración local visible;
5. símbolo exportado por un módulo importado;
6. documentación registrada en el catálogo.

## Límite técnico actual

Este paquete es una mejora fuerte sobre la resolución por regex, pero sigue usando `NcSourceIndex` como puente textual. La arquitectura profesional definitiva debe trasladar los identificadores de nC a PSI tipado e implementar `PsiReference` o `PsiPolyVariantReference`.

Eso permitirá reutilizar una sola semántica para:

- Ctrl+B y Ctrl+clic;
- Rename;
- Find Usages;
- Quick Definition;
- Quick Documentation;
- autocompletado semántico;
- inspecciones.

JetBrains recomienda referencias PSI como mecanismo principal de resolución. `GotoDeclarationHandler` debe considerarse una capa de compatibilidad mientras el PSI de nC madura.

## Fallo `compileTestKotlin`: `BasePlatformTestCase` no resuelto

La versión 2 del paquete incluye un corrector automático porque copiar las pruebas sin añadir sus dependencias deja `BasePlatformTestCase` y `myFixture` fuera del classpath.

```powershell
.\scripts\Fix-NcPluginTestFramework.ps1 `
    -PluginRoot $PluginRoot `
    -Build `
    -RefreshDependencies
```

El instalador principal también aplica esta corrección automáticamente salvo que se use `-SkipTests`.

Los warnings de `NcTextAttributes.kt` son deprecaciones, no errores de compilación. Se explican en `BUILD_FAILURE_FIX.md`.
