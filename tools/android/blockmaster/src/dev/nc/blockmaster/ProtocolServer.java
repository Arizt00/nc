package dev.nc.blockmaster;

import android.content.Context;
import android.util.Log;
import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.SocketException;
import java.net.SocketTimeoutException;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import javax.net.ssl.SSLServerSocket;

final class ProtocolServer implements AutoCloseable {
    static final int LEGACY_PORT = 8080;
    static final int SECURE_PORT = 8443;
    private static final String TAG = "BlockMasterProtocol";
    private static final int SOCKET_TIMEOUT_MS = 30_000;

    private final Context context;
    private final String token;
    private final BlocklistStore store;
    private final AtomicBoolean running = new AtomicBoolean(false);
    private final ExecutorService acceptors = Executors.newFixedThreadPool(2);
    private final ExecutorService clients = Executors.newCachedThreadPool();
    private volatile ServerSocket secureServer;
    private volatile ServerSocket legacyServer;

    ProtocolServer(Context context, String token, BlocklistStore store) {
        this.context = context.getApplicationContext();
        this.token = token;
        this.store = store;
    }

    void start() throws Exception {
        if (!running.compareAndSet(false, true)) return;
        // El control compatible queda limitado al propio dispositivo/ADB USB.
        if (BuildConfig.LEGACY_USB_CONTROL_ENABLED) {
            legacyServer = new ServerSocket(LEGACY_PORT, 16, InetAddress.getLoopbackAddress());
            legacyServer.setReuseAddress(true);
            acceptors.execute(() -> acceptLegacyLoop(legacyServer));
        }
        secureServer = secureSocket();
        acceptors.execute(() -> acceptLoop(secureServer, true));
    }

    private SSLServerSocket secureSocket() throws Exception {
        SSLServerSocket server = (SSLServerSocket) SecurityVault.createTlsServerContext()
                .getServerSocketFactory()
                .createServerSocket(SECURE_PORT, 32, InetAddress.getByName("0.0.0.0"));
        server.setEnabledProtocols(new String[] {"TLSv1.3", "TLSv1.2"});
        server.setReuseAddress(true);
        return server;
    }

    private void acceptLoop(ServerSocket server, boolean secure) {
        while (running.get()) {
            try {
                Socket socket = server.accept();
                socket.setSoTimeout(SOCKET_TIMEOUT_MS);
                socket.setKeepAlive(true);
                socket.setTcpNoDelay(true);
                clients.execute(() -> handle(socket, secure));
            } catch (SocketException closed) {
                if (running.get()) Log.e(TAG, "accept failed", closed);
            } catch (Exception error) {
                Log.e(TAG, "accept failed", error);
            }
        }
    }

    private void acceptLegacyLoop(ServerSocket server) {
        while (running.get()) {
            try {
                Socket socket = server.accept();
                socket.setSoTimeout(SOCKET_TIMEOUT_MS);
                socket.setKeepAlive(false);
                socket.setTcpNoDelay(true);
                clients.execute(() -> handleLegacy(socket));
            } catch (SocketException closed) {
                if (running.get()) Log.e(TAG, "legacy accept failed", closed);
            } catch (Exception error) {
                Log.e(TAG, "legacy accept failed", error);
            }
        }
    }

    private void handleLegacy(Socket socket) {
        try (Socket peer = socket;
             InputStream input = new BufferedInputStream(peer.getInputStream());
             OutputStream output = new BufferedOutputStream(peer.getOutputStream())) {
            ByteArrayOutputStream request = new ByteArrayOutputStream();
            byte[] chunk = new byte[1024];
            while (request.size() <= 4096) {
                try {
                    int read = input.read(chunk, 0, Math.min(chunk.length, 4097 - request.size()));
                    if (read < 0) break;
                    request.write(chunk, 0, read);
                    peer.setSoTimeout(150);
                } catch (SocketTimeoutException completeRequest) {
                    if (request.size() > 0) break;
                    throw completeRequest;
                }
            }
            if (request.size() == 0) return;
            String response;
            if (request.size() > 4096) {
                response = "ERR|REQUEST_TOO_LARGE";
            } else {
                String command = new String(request.toByteArray(), java.nio.charset.StandardCharsets.UTF_8);
                response = execute(command.trim());
            }
            output.write(response.getBytes(java.nio.charset.StandardCharsets.UTF_8));
            output.flush();
        } catch (Exception error) {
            Log.i(TAG, "legacy client disconnected: " + error.getMessage());
        }
    }

    private void handle(Socket socket, boolean secure) {
        try (Socket peer = socket;
             InputStream input = new BufferedInputStream(peer.getInputStream());
             OutputStream output = new BufferedOutputStream(peer.getOutputStream())) {
            String serverNonce = ProtocolSecurity.nonce();
            FrameCodec.write(output, FrameCodec.Framing.LINE,
                    "HELLO|NCBM1|" + serverNonce + "|LINE,FRAME|" + (secure ? "SECURE" : "RAW"));
            String selection = FrameCodec.readLine(input);
            String[] fields = selection.split("\\|", -1);
            if (secure) handleSecure(input, output, fields, serverNonce);
            else throw new SecurityException("raw protocol disabled");
        } catch (Exception error) {
            Log.i(TAG, "client disconnected: " + error.getMessage());
        }
    }

    private void handleSecure(InputStream input, OutputStream output, String[] auth,
                              String serverNonce) throws Exception {
        if (auth.length != 4 || !"AUTH".equals(auth[0])) throw new SecurityException("bad AUTH frame");
        FrameCodec.Framing framing = FrameCodec.Framing.parse(auth[1]);
        String clientNonce = auth[2];
        String signed = "AUTH|" + auth[1].toUpperCase() + "|" + serverNonce + "|" + clientNonce;
        if (!ProtocolSecurity.equalHex(ProtocolSecurity.hmac(token, signed), auth[3])) {
            FrameCodec.write(output, FrameCodec.Framing.LINE, "AUTH_ERR");
            throw new SecurityException("authentication failed");
        }
        String session = ProtocolSecurity.nonce();
        String ready = "AUTH_OK|" + session;
        FrameCodec.write(output, FrameCodec.Framing.LINE,
                ready + "|" + ProtocolSecurity.hmac(token, ready));
        long expectedSequence = 1;
        while (running.get()) {
            String frame = FrameCodec.read(input, framing);
            String[] request = frame.split("\\|", 5);
            if (request.length != 5 || !"REQ".equals(request[0]) || !session.equals(request[1])) {
                throw new SecurityException("bad request envelope");
            }
            long sequence = Long.parseLong(request[2]);
            if (sequence != expectedSequence) throw new SecurityException("replay or sequence gap");
            String signedRequest = "REQ|" + session + "|" + sequence + "|" + request[3];
            if (!ProtocolSecurity.equalHex(ProtocolSecurity.hmac(token, signedRequest), request[4])) {
                throw new SecurityException("request HMAC mismatch");
            }
            String command = ProtocolSecurity.decode(request[3]);
            String response = execute(command);
            String payload = ProtocolSecurity.encode(response);
            String envelope = "RES|" + session + "|" + sequence + "|" + payload;
            FrameCodec.write(output, framing, envelope + "|" + ProtocolSecurity.hmac(token, envelope));
            expectedSequence++;
            if (isShutdown(command)) break;
        }
    }

    private synchronized String execute(String command) {
        String response = NativeBridge.command(command);
        if (isMutation(command) && response.startsWith("OK|")) {
            try {
                store.save();
                if (!persistConfiguration(command)) return "ERR|PERSISTENCE|SharedPreferences";
            } catch (Exception error) {
                return "ERR|PERSISTENCE|" + error.getClass().getSimpleName();
            }
        }
        if (isShutdown(command)) close();
        return response;
    }

    private static boolean isMutation(String command) {
        String value = command.trim().toUpperCase(Locale.ROOT);
        return value.startsWith("AGREGAR ") || value.startsWith("ELIMINAR ")
                || value.startsWith("BLOQUEAR_POR_PATRON ")
                || value.startsWith("DESBLOQUEAR_PATRON ")
                || value.startsWith("AGREGAR_WHITELIST ")
                || value.startsWith("ELIMINAR_WHITELIST ")
                || value.startsWith("AJUSTAR_TIEMPO ")
                || value.startsWith("MODO_WHITELIST ")
                || value.startsWith("MODO_CONTACTOS ")
                || value.startsWith("PERMITIR_POR_PATRON ")
                || value.startsWith("QUITAR_PERMISO_PATRON ")
                || value.startsWith("MODO_PROTECCION ")
                || value.startsWith("BLOQUEAR_OCULTOS ")
                || value.startsWith("APRENDIZAJE_ADAPTATIVO ")
                || value.startsWith("IA_FEEDBACK ")
                || "IA_REINICIAR".equals(value) || "LIMPIAR".equals(value);
    }

    private boolean persistConfiguration(String command) {
        String value = command.trim().toUpperCase(Locale.ROOT);
        android.content.SharedPreferences.Editor editor = context
                .getSharedPreferences(BlockMasterService.PREFS, Context.MODE_PRIVATE).edit();
        if (value.startsWith("AJUSTAR_TIEMPO ")) {
            editor.putInt(BlockMasterService.PREF_URGENCY_SECONDS,
                    Integer.parseInt(value.substring("AJUSTAR_TIEMPO ".length()).trim()));
            return editor.commit();
        }
        if (value.startsWith("MODO_WHITELIST ")) {
            editor.putBoolean(BlockMasterService.PREF_WHITELIST_MODE,
                    "ON".equals(value.substring("MODO_WHITELIST ".length()).trim()));
            return editor.commit();
        }
        if (value.startsWith("MODO_CONTACTOS ")) {
            editor.putBoolean(BlockMasterService.PREF_CONTACTS_ONLY,
                    "ON".equals(value.substring("MODO_CONTACTOS ".length()).trim()));
            return editor.commit();
        }
        return true;
    }

    private static boolean isShutdown(String command) {
        return "APAGAR_SERVIDOR".equalsIgnoreCase(command.trim());
    }

    @Override public void close() {
        if (!running.compareAndSet(true, false)) return;
        try { if (secureServer != null) secureServer.close(); } catch (Exception ignored) {}
        try { if (legacyServer != null) legacyServer.close(); } catch (Exception ignored) {}
        clients.shutdownNow();
        acceptors.shutdownNow();
    }
}
