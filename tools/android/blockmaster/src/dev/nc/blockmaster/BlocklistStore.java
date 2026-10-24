package dev.nc.blockmaster;

import android.content.Context;
import android.util.Log;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

final class BlocklistStore {
    private final Context context;
    private final File target;

    BlocklistStore(Context context) {
        this.context = context.getApplicationContext();
        target = new File(this.context.getFilesDir(), "blockmaster.snapshot");
    }

    synchronized void load() throws IOException {
        if (!target.isFile()) return;
        byte[] bytes = new byte[(int) target.length()];
        try (FileInputStream input = new FileInputStream(target)) {
            int offset = 0;
            while (offset < bytes.length) {
                int read = input.read(bytes, offset, bytes.length - offset);
                if (read < 0) break;
                offset += read;
            }
        }
        String stored = new String(bytes, StandardCharsets.UTF_8);
        String snapshot;
        try {
            snapshot = SecurityVault.isEncrypted(stored)
                    ? SecurityVault.decrypt(context, "rule-snapshot", stored) : stored;
        } catch (Exception authenticationFailure) {
            archiveIncompatible("authentication");
            return;
        }
        if (!NativeBridge.restore(snapshot)) {
            archiveIncompatible("schema");
        } else if (!SecurityVault.isEncrypted(stored) || SecurityVault.needsMigration(stored)) {
            save();
        }
    }

    private void archiveIncompatible(String reason) throws IOException {
            File archived = new File(target.getParentFile(),
                    target.getName() + ".incompatible." + reason + "." + System.currentTimeMillis());
            Files.move(target.toPath(), archived.toPath(), StandardCopyOption.REPLACE_EXISTING);
            Log.w("BlockMasterStore", "Archived incompatible snapshot as " + archived.getName());
    }

    synchronized void save() throws IOException {
        String snapshot = NativeBridge.snapshot();
        String protectedSnapshot;
        try {
            protectedSnapshot = SecurityVault.encrypt(context, "rule-snapshot", snapshot);
        } catch (Exception error) {
            throw new IOException("could not encrypt nC rule snapshot", error);
        }
        File temp = new File(target.getParentFile(), target.getName() + ".tmp");
        try (FileOutputStream output = new FileOutputStream(temp, false)) {
            output.write(protectedSnapshot.getBytes(StandardCharsets.UTF_8));
            output.flush();
            output.getFD().sync();
        }
        try {
            Files.move(temp.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING,
                    StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException noAtomicMove) {
            Files.move(temp.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING);
        }
    }

    static void eraseAll(Context context) throws IOException {
        File directory = context.getApplicationContext().getFilesDir();
        File[] files = directory.listFiles((parent, name) ->
                name.equals("blockmaster.snapshot")
                        || name.startsWith("blockmaster.snapshot."));
        if (files == null) return;
        for (File file : files) {
            if (file.exists() && !file.delete()) {
                throw new IOException("could not erase " + file.getName());
            }
        }
    }
}
