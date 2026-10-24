package dev.nc.blockmaster;

import android.content.Context;
import android.content.SharedPreferences;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;

/** Encrypted, device-local journal of decisions made by the nC call core. */
final class ActivityStore {
    private static final String PREFS = "uncall.nc.activity.v1";
    private static final String KEY_LOG = "encrypted_activity_log";
    private static final String PURPOSE = "activity-log";
    private static final int MAX_RECORDS = 250;

    static final class Record {
        final long wallTime;
        final String decision;

        Record(long wallTime, String decision) {
            this.wallTime = wallTime;
            this.decision = decision == null ? "" : decision;
        }
    }

    private ActivityStore() {}

    static synchronized List<Record> load(Context context) {
        SharedPreferences preferences = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        String envelope = preferences.getString(KEY_LOG, "");
        if (!SecurityVault.isEncrypted(envelope)) return new ArrayList<>();
        try {
            return decode(SecurityVault.decrypt(context, PURPOSE, envelope));
        } catch (Exception invalid) {
            return new ArrayList<>();
        }
    }

    static synchronized void append(Context context, long wallTime, String decision)
            throws Exception {
        List<Record> records = load(context);
        records.add(new Record(wallTime, decision));
        while (records.size() > MAX_RECORDS) records.remove(0);
        String envelope = SecurityVault.encrypt(context, PURPOSE, encode(records));
        if (!context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
                .putString(KEY_LOG, envelope).commit()) {
            throw new IllegalStateException("activity journal persistence failed");
        }
    }

    static synchronized void eraseAll(Context context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().clear().commit();
    }

    private static String encode(List<Record> records) {
        StringBuilder payload = new StringBuilder();
        for (Record record : records) {
            if (payload.length() > 0) payload.append('\n');
            payload.append(record.wallTime).append(':')
                    .append(Base64.getUrlEncoder().withoutPadding().encodeToString(
                            record.decision.getBytes(StandardCharsets.UTF_8)));
        }
        return payload.toString();
    }

    private static List<Record> decode(String payload) {
        List<Record> records = new ArrayList<>();
        if (payload == null || payload.isEmpty()) return records;
        for (String line : payload.split("\\n", -1)) {
            int separator = line.indexOf(':');
            if (separator < 1) continue;
            try {
                long wallTime = Long.parseLong(line.substring(0, separator));
                String decision = new String(Base64.getUrlDecoder().decode(
                        line.substring(separator + 1)), StandardCharsets.UTF_8);
                if (!decision.isEmpty()) records.add(new Record(wallTime, decision));
            } catch (Exception malformed) {
                // Skip a malformed record without discarding the authenticated journal.
            }
        }
        return records;
    }
}
