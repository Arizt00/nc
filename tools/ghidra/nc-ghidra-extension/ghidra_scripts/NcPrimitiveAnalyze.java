//@category nC
//@menupath Tools.nC Primitive Analyze

import ghidra.app.script.GhidraScript;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

public class NcPrimitiveAnalyze extends GhidraScript {

    private String readAll(Process process) throws Exception {
        StringBuilder sb = new StringBuilder();
        try (BufferedReader br = new BufferedReader(new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
            String line;
            while ((line = br.readLine()) != null) {
                sb.append(line).append("\n");
            }
        }
        return sb.toString();
    }

    @Override
    protected void run() throws Exception {
        String sourcePath = askString("nC Analyze", "Ruta archivo nC (.ncp/.nm/.nb/.n):");
        if (sourcePath == null || sourcePath.trim().isEmpty()) {
            printerr("Ruta vacia");
            return;
        }

        String profile = askChoice("nC Profile", "Perfil editorial:", java.util.List.of("novice", "standard", "expert"), "expert");

        ProcessBuilder routePb = new ProcessBuilder(
            "ncc-tools",
            "route-ghidra",
            "--file",
            sourcePath,
            "--profile",
            profile,
            "--json"
        );
        routePb.redirectErrorStream(true);
        Process routeProc = routePb.start();
        String routeOutput = readAll(routeProc);
        int routeCode = routeProc.waitFor();
        if (routeCode != 0) {
            printerr("Router failed: " + routeOutput);
            return;
        }

        println("=== nC Route ===");
        println(routeOutput.trim());

        ProcessBuilder analyzePb = new ProcessBuilder(
            "nc-sys-lsp",
            "analyze",
            sourcePath,
            "--profile",
            profile,
            "--migration-stage",
            "strict",
            "--json"
        );
        analyzePb.redirectErrorStream(true);
        Process analyzeProc = analyzePb.start();
        String analyzeOutput = readAll(analyzeProc);
        int analyzeCode = analyzeProc.waitFor();

        println("=== nC Low-level Diagnostics ===");
        println(analyzeOutput.trim());

        if (analyzeCode != 0) {
            printerr("nc-sys-lsp analyze failed");
        }
    }
}
