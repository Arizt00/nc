package dev.nc.blockmaster;

import android.content.Context;
import android.content.SharedPreferences;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Base64;

/** Encrypted presentation metadata attached to rules owned by the nC core. */
final class RuleProfileStore {
    private static final String PREFS = "uncall.nc.rule.profiles.v1";
    private static final String PURPOSE_PREFIX = "rule-profile:";

    static final class Profile {
        static final Profile EMPTY = new Profile("", "", "");

        final String name;
        final String address;
        final String notes;

        Profile(String name, String address, String notes) {
            this.name = clean(name);
            this.address = clean(address);
            this.notes = clean(notes);
        }

        boolean isEmpty() {
            return name.isEmpty() && address.isEmpty() && notes.isEmpty();
        }
    }

    private RuleProfileStore() {}

    static Profile load(Context context, String group, String rule) {
        String key = key(group, rule);
        String envelope = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .getString(key, "");
        if (!SecurityVault.isEncrypted(envelope)) return Profile.EMPTY;
        try {
            String payload = SecurityVault.decrypt(context, PURPOSE_PREFIX + key, envelope);
            String[] fields = payload.split("\\.", -1);
            if (fields.length != 3) return Profile.EMPTY;
            return new Profile(decode(fields[0]), decode(fields[1]), decode(fields[2]));
        } catch (Exception invalid) {
            return Profile.EMPTY;
        }
    }

    static void save(Context context, String group, String rule,
            String name, String address, String notes) throws Exception {
        String key = key(group, rule);
        Profile profile = new Profile(name, address, notes);
        SharedPreferences preferences = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        if (profile.isEmpty()) {
            if (!preferences.edit().remove(key).commit()) {
                throw new IllegalStateException("rule profile removal failed");
            }
            return;
        }
        String payload = encode(profile.name) + "." + encode(profile.address)
                + "." + encode(profile.notes);
        String envelope = SecurityVault.encrypt(context, PURPOSE_PREFIX + key, payload);
        if (!preferences.edit().putString(key, envelope).commit()) {
            throw new IllegalStateException("rule profile persistence failed");
        }
    }

    static void delete(Context context, String group, String rule) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .edit().remove(key(group, rule)).commit();
    }

    static void eraseAll(Context context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().clear().commit();
    }

    private static String key(String group, String rule) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest((clean(group) + "\u0000" + clean(rule))
                    .getBytes(StandardCharsets.UTF_8));
            StringBuilder text = new StringBuilder("profile_");
            for (byte value : hash) text.append(String.format("%02x", value & 0xff));
            return text.toString();
        } catch (Exception unavailable) {
            throw new IllegalStateException("SHA-256 unavailable", unavailable);
        }
    }

    private static String encode(String value) {
        return Base64.getUrlEncoder().withoutPadding()
                .encodeToString(clean(value).getBytes(StandardCharsets.UTF_8));
    }

    private static String decode(String value) {
        if (value.isEmpty()) return "";
        return new String(Base64.getUrlDecoder().decode(value), StandardCharsets.UTF_8);
    }

    private static String clean(String value) {
        return value == null ? "" : value.trim();
    }
}
