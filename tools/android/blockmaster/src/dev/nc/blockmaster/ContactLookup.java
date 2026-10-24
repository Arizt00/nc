package dev.nc.blockmaster;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.database.Cursor;
import android.net.Uri;
import android.provider.ContactsContract;
import android.util.Log;

final class ContactLookup {
    static final int UNAVAILABLE = -1;
    static final int UNKNOWN = 0;
    static final int CONTACT = 1;

    private ContactLookup() {}

    static int classify(Context context, String number) {
        String result = displayName(context, number, true);
        return result == null ? UNAVAILABLE : result.isEmpty() ? UNKNOWN : CONTACT;
    }

    static String displayName(Context context, String number) {
        String result = displayName(context, number, false);
        return result == null ? "" : result;
    }

    private static String displayName(Context context, String number, boolean signalUnavailable) {
        if (!BuildConfig.NATIVE_CONTACTS) return signalUnavailable ? null : "";
        if (context.checkSelfPermission(Manifest.permission.READ_CONTACTS)
                != PackageManager.PERMISSION_GRANTED) {
            return signalUnavailable ? null : "";
        }
        if (number == null || number.trim().isEmpty()) return "";

        Uri lookup = Uri.withAppendedPath(
                ContactsContract.PhoneLookup.CONTENT_FILTER_URI, Uri.encode(number));
        try (Cursor cursor = context.getContentResolver().query(
                lookup,
                new String[] {ContactsContract.PhoneLookup._ID,
                        ContactsContract.PhoneLookup.DISPLAY_NAME},
                null,
                null,
                null)) {
            if (cursor == null || !cursor.moveToFirst()) return "";
            String name = cursor.getString(1);
            String cleanName = name == null ? "" : name.trim();
            return cleanName.isEmpty() && signalUnavailable ? "contact" : cleanName;
        } catch (RuntimeException error) {
            Log.e("BlockMasterContacts", "contact lookup unavailable", error);
            return signalUnavailable ? null : "";
        }
    }
}
