package dev.nc.blockmaster;

import android.content.Context;
import android.util.Log;
import java.io.IOException;

public final class NativeBridge {
    private static boolean initialized;

    static {
        System.loadLibrary("ncblockmaster");
    }

    private NativeBridge() {}

    public static synchronized void ensureInitialized(Context context) throws IOException {
        if (initialized) return;
        try {
            SecurityVault.assertTrustedSigner(context);
        } catch (Exception error) {
            throw new IOException("UnCall signing identity verification failed", error);
        }
        if (!init()) throw new IOException("native nC core initialization failed");
        Context application = context.getApplicationContext();
        new BlocklistStore(application).load();
        try {
            SignedSpamDatabase.verifyAndInstall();
        } catch (Exception verificationFailure) {
            invalidateSignedSpamBase("VERIFY_FAILED");
            Log.e("UnCall", "Signed spam data disabled; the core remains fail-open",
                    verificationFailure);
        }
        if ("store-release".equals(BuildConfig.DISTRIBUTION)) {
            String response = command("MODO_WHITELIST OFF");
            if (!response.startsWith("OK|")) {
                throw new IOException("store fail-open mode initialization failed: " + response);
            }
        }
        if (BuildConfig.NATIVE_CONTACTS) {
            boolean contactsOnly = application
                    .getSharedPreferences(BlockMasterService.PREFS, Context.MODE_PRIVATE)
                    .getBoolean(BlockMasterService.PREF_CONTACTS_ONLY, true);
            String response = command("MODO_CONTACTOS " + (contactsOnly ? "ON" : "OFF"));
            if (!response.startsWith("OK|")) {
                throw new IOException("native contacts mode initialization failed: " + response);
            }
        }
        initialized = true;
    }

    public static native boolean init();
    public static native String command(String command);
    public static native boolean shouldBlock(String number);
    public static native boolean shouldBlockAt(String number, int monotonicSeconds);
    public static native boolean shouldBlockContactAt(
            String number, int monotonicSeconds, boolean contact);
    public static native String lastDecision();
    public static native boolean restore(String snapshot);
    public static native String snapshot();
    public static native boolean callSeen(boolean blocked);
    public static native boolean beginSignedSpamBase(
            String version, String keyId, int issued, int expires, int count);
    public static native boolean addSignedSpamIndicator(
            String type, String value, int risk, String label, String action);
    public static native boolean commitSignedSpamBase(String digest);
    public static native boolean invalidateSignedSpamBase(String reason);
    public static native String abiReport();
}
