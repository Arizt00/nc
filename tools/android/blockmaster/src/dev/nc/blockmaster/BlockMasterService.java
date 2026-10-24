package dev.nc.blockmaster;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;

public final class BlockMasterService extends Service {
    static final String PREFS = "blockmaster";
    static final String PREF_TOKEN = "protocol_token";
    static final String PREF_URGENCY_SECONDS = "urgency_seconds";
    static final String PREF_WHITELIST_MODE = "whitelist_mode";
    static final String PREF_CONTACTS_ONLY = "contacts_only";
    static final String PREF_LAST_DECISION = "last_decision";
    static final String PREF_LAST_DECISION_AT = "last_decision_at";
    static final String ACTION_STOP = "dev.nc.blockmaster.STOP";
    private static final String CHANNEL = "blockmaster.remote";
    private static final int NOTIFICATION_ID = 2401;
    private ProtocolServer server;

    @Override public void onCreate() {
        super.onCreate();
        if (!BuildConfig.REMOTE_CONTROL_ENABLED) {
            stopSelf();
            return;
        }
        ensureChannel();
        startForeground(NOTIFICATION_ID, notification(localized(
                "Inicializando motor nC", "Initializing nC engine")));
        try {
            SecurityVault.assertTrustedSigner(this);
            NativeBridge.ensureInitialized(this);
            BlocklistStore store = new BlocklistStore(this);
            SharedPreferences prefs = getSharedPreferences(PREFS, MODE_PRIVATE);
            String token = SecurityVault.getOrCreateAccessToken(this, prefs);
            NativeBridge.command("AJUSTAR_TIEMPO " +
                    prefs.getInt(PREF_URGENCY_SECONDS, 60));
            NativeBridge.command("MODO_WHITELIST " +
                    (prefs.getBoolean(PREF_WHITELIST_MODE, true) ? "ON" : "OFF"));
            if (BuildConfig.NATIVE_CONTACTS) {
                NativeBridge.command("MODO_CONTACTOS " +
                        (prefs.getBoolean(PREF_CONTACTS_ONLY, true) ? "ON" : "OFF"));
            }
            server = new ProtocolServer(this, token, store);
            server.start();
            NotificationManager manager = getSystemService(NotificationManager.class);
            String endpoint = "TLS :" + ProtocolServer.SECURE_PORT;
            if (BuildConfig.LEGACY_USB_CONTROL_ENABLED) {
                endpoint = "USB/localhost :" + ProtocolServer.LEGACY_PORT + "  " + endpoint;
            }
            manager.notify(NOTIFICATION_ID, notification(endpoint));
        } catch (Exception error) {
            Log.e("BlockMasterService", "startup failed", error);
            stopSelf();
        }
    }

    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) stopSelf();
        return START_STICKY;
    }

    @Override public void onDestroy() {
        if (server != null) server.close();
        super.onDestroy();
    }

    @Override public IBinder onBind(Intent intent) {
        return null;
    }

    private Notification notification(String text) {
        Intent open = new Intent(this, MainActivity.class);
        PendingIntent pending = PendingIntent.getActivity(this, 0, open,
                PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
        return new Notification.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.stat_sys_phone_call)
                .setContentTitle("UnCall")
                .setContentText(text)
                .setContentIntent(pending)
                .setOngoing(true)
                .build();
    }

    private void ensureChannel() {
        NotificationManager manager = getSystemService(NotificationManager.class);
        manager.createNotificationChannel(new NotificationChannel(CHANNEL,
                "UnCall · " + localized("protección de llamadas", "Call protection"),
                NotificationManager.IMPORTANCE_LOW));
    }

    private String localized(String spanish, String english) {
        SharedPreferences prefs = getSharedPreferences(PREFS, MODE_PRIVATE);
        String mode = prefs.getString("ui_language_mode", "system");
        if ("system".equals(mode)) mode = Translations.systemLanguage();
        if ("es".equals(mode)) return spanish;
        if ("en".equals(mode)) return english;
        return Translations.translate(mode, english);
    }
}
