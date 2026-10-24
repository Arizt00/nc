package dev.nc.blockmaster;

import android.net.Uri;
import android.telecom.Call;
import android.telecom.CallScreeningService;
import android.os.SystemClock;
import android.util.Log;

public final class NcCallScreeningService extends CallScreeningService {
    @Override public void onScreenCall(Call.Details details) {
        try {
            NativeBridge.ensureInitialized(this);
        } catch (Exception error) {
            Log.e("NcCallScreening", "native state initialization failed", error);
            rememberDecision("ALLOWED|INITIALIZATION_ERROR|");
            respondToCall(details, new CallResponse.Builder().setDisallowCall(false).build());
            return;
        }
        Uri handle = details.getHandle();
        String number = handle == null ? "" : handle.getSchemeSpecificPart();
        int now = (int) Math.min(Integer.MAX_VALUE, SystemClock.elapsedRealtime() / 1000L);
        int contactState = ContactLookup.classify(this, number);
        boolean blocked = BuildConfig.NATIVE_CONTACTS
                && contactState != ContactLookup.UNAVAILABLE
                ? NativeBridge.shouldBlockContactAt(
                        number, now, contactState == ContactLookup.CONTACT)
                : NativeBridge.shouldBlockAt(number, now);
        String decision = BuildConfig.NATIVE_CONTACTS
                ? NativeBridge.lastDecision()
                : (blocked ? "BLOCKED|LEGACY|" : "ALLOWED|LEGACY|") + number;
        rememberDecision(decision);
        NativeBridge.callSeen(blocked);
        CallResponse.Builder response = new CallResponse.Builder();
        if (blocked) {
            response.setDisallowCall(true)
                    .setRejectCall(true)
                    .setSilenceCall(true)
                    .setSkipCallLog(false)
                    .setSkipNotification(false);
        } else {
            response.setDisallowCall(false);
        }
        respondToCall(details, response.build());
    }

    private void rememberDecision(String decision) {
        long wallTime = System.currentTimeMillis();
        String protectedDecision;
        try {
            protectedDecision = SecurityVault.encrypt(this, "last-decision", decision);
        } catch (Exception error) {
            Log.e("NcCallScreening", "decision encryption failed", error);
            protectedDecision = "ALLOWED|STORAGE_ERROR|";
        }
        getSharedPreferences(BlockMasterService.PREFS, MODE_PRIVATE).edit()
                .putString(BlockMasterService.PREF_LAST_DECISION, protectedDecision)
                .putLong(BlockMasterService.PREF_LAST_DECISION_AT, wallTime)
                .apply();
        try {
            ActivityStore.append(this, wallTime, decision);
        } catch (Exception persistenceFailure) {
            Log.e("NcCallScreening", "activity journal persistence failed", persistenceFailure);
        }
    }
}
