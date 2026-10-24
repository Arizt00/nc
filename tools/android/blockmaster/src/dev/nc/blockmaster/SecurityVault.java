package dev.nc.blockmaster;

import android.content.Context;
import android.content.SharedPreferences;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.os.Build;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.security.KeyPairGenerator;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.Date;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;
import javax.net.ssl.KeyManagerFactory;
import javax.net.ssl.SSLContext;
import javax.security.auth.x500.X500Principal;

/** Device-bound cryptographic provider for the nC Android runtime. */
final class SecurityVault {
    private static final String KEY_ALIAS = "uncall.nc.v1.device.aes";
    private static final String TLS_KEY_ALIAS = "uncall.nc.v1.device.tls";
    private static final String ENVELOPE = "NCV2:";
    private static final String LEGACY_ENVELOPE = "NCV1:";
    private static final String VAULT_SCHEMA = "uncall-vault-v1";

    private SecurityVault() {}

    static void assertTrustedSigner(Context context) throws Exception {
        PackageManager manager = context.getPackageManager();
        PackageInfo info;
        Signature[] signatures;
        if (Build.VERSION.SDK_INT >= 28) {
            info = manager.getPackageInfo(context.getPackageName(),
                    PackageManager.GET_SIGNING_CERTIFICATES);
            signatures = info.signingInfo.getApkContentsSigners();
        } else {
            info = manager.getPackageInfo(context.getPackageName(), PackageManager.GET_SIGNATURES);
            signatures = info.signatures;
        }
        if (signatures == null || signatures.length == 0) {
            throw new SecurityException("unexpected signer set");
        }
        String[] trusted = BuildConfig.TRUSTED_SIGNING_CERTS_SHA256.toLowerCase().split(";", -1);
        if (trusted.length == 0 || trusted[0].isEmpty()) {
            throw new SecurityException("trusted signer policy is empty");
        }
        for (Signature signature : signatures) {
            String actual = hex(MessageDigest.getInstance("SHA-256")
                    .digest(signature.toByteArray()));
            boolean accepted = false;
            for (String candidate : trusted) {
                if (candidate.length() == 64 && MessageDigest.isEqual(
                        actual.getBytes(StandardCharsets.US_ASCII),
                        candidate.getBytes(StandardCharsets.US_ASCII))) {
                    accepted = true;
                    break;
                }
            }
            if (!accepted) throw new SecurityException("signing identity mismatch");
        }
    }

    static SSLContext createTlsServerContext() throws Exception {
        KeyStore store = androidKeyStore();
        ensureTlsIdentity(store);
        KeyManagerFactory managers = KeyManagerFactory.getInstance(
                KeyManagerFactory.getDefaultAlgorithm());
        managers.init(store, null);
        SSLContext tls = SSLContext.getInstance("TLS");
        tls.init(managers.getKeyManagers(), null, new SecureRandom());
        return tls;
    }

    static String tlsCertificateSha256() throws Exception {
        KeyStore store = androidKeyStore();
        ensureTlsIdentity(store);
        return hex(MessageDigest.getInstance("SHA-256")
                .digest(store.getCertificate(TLS_KEY_ALIAS).getEncoded()));
    }

    static void destroyDeviceSecrets() throws Exception {
        KeyStore store = androidKeyStore();
        if (store.containsAlias(KEY_ALIAS)) store.deleteEntry(KEY_ALIAS);
        if (store.containsAlias(TLS_KEY_ALIAS)) store.deleteEntry(TLS_KEY_ALIAS);
    }

    static String getOrCreateAccessToken(Context context, SharedPreferences prefs) throws Exception {
        String stored = prefs.getString(BlockMasterService.PREF_TOKEN, "");
        if (isEncrypted(stored)) {
            String token = decrypt(context, "access-token", stored);
            if (needsMigration(stored) && !prefs.edit().putString(BlockMasterService.PREF_TOKEN,
                    encrypt(context, "access-token", token)).commit()) {
                throw new SecurityException("token migration failed");
            }
            return token;
        }
        String token = stored.isEmpty()
                ? ProtocolSecurity.nonce() + ProtocolSecurity.nonce() : stored;
        if (!prefs.edit().putString(BlockMasterService.PREF_TOKEN,
                encrypt(context, "access-token", token)).commit()) {
            throw new SecurityException("token persistence failed");
        }
        return token;
    }

    static String encrypt(Context context, String purpose, String plaintext) throws Exception {
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key());
        cipher.updateAAD(aad(context, purpose));
        byte[] iv = cipher.getIV();
        byte[] ciphertext = cipher.doFinal(plaintext.getBytes(StandardCharsets.UTF_8));
        return ENVELOPE + Base64.getUrlEncoder().withoutPadding().encodeToString(iv)
                + "." + Base64.getUrlEncoder().withoutPadding().encodeToString(ciphertext);
    }

    static String decrypt(Context context, String purpose, String envelope) throws Exception {
        if (envelope == null || !isEncrypted(envelope)) {
            throw new SecurityException("invalid vault envelope");
        }
        boolean legacy = envelope.startsWith(LEGACY_ENVELOPE);
        String prefix = legacy ? LEGACY_ENVELOPE : ENVELOPE;
        String[] fields = envelope.substring(prefix.length()).split("\\.", -1);
        if (fields.length != 2) throw new SecurityException("invalid vault fields");
        byte[] iv = Base64.getUrlDecoder().decode(fields[0]);
        if (iv.length != 12) throw new SecurityException("invalid vault IV");
        byte[] ciphertext = Base64.getUrlDecoder().decode(fields[1]);
        if (!legacy) return decryptWithAad(iv, ciphertext, aad(context, purpose));
        Exception last = null;
        for (String buildId : BuildConfig.LEGACY_VAULT_BUILD_IDS.split(";", -1)) {
            if (buildId.isEmpty()) continue;
            try {
                return decryptWithAad(iv, ciphertext, legacyAad(context, purpose, buildId));
            } catch (Exception authenticationFailure) {
                last = authenticationFailure;
            }
        }
        throw new SecurityException("legacy vault authentication failed", last);
    }

    static boolean isEncrypted(String value) {
        return value != null
                && (value.startsWith(ENVELOPE) || value.startsWith(LEGACY_ENVELOPE));
    }

    static boolean needsMigration(String value) {
        return value != null && value.startsWith(LEGACY_ENVELOPE);
    }

    private static SecretKey key() throws Exception {
        KeyStore store = androidKeyStore();
        SecretKey existing = (SecretKey) store.getKey(KEY_ALIAS, null);
        if (existing != null) return existing;
        KeyGenerator generator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        generator.init(new KeyGenParameterSpec.Builder(KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .build());
        return generator.generateKey();
    }

    private static KeyStore androidKeyStore() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        return store;
    }

    private static void ensureTlsIdentity(KeyStore store) throws Exception {
        if (store.containsAlias(TLS_KEY_ALIAS)) return;
        long now = System.currentTimeMillis();
        KeyPairGenerator generator = KeyPairGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_RSA, "AndroidKeyStore");
        generator.initialize(new KeyGenParameterSpec.Builder(TLS_KEY_ALIAS,
                KeyProperties.PURPOSE_SIGN | KeyProperties.PURPOSE_VERIFY)
                .setKeySize(3072)
                .setDigests(KeyProperties.DIGEST_SHA256, KeyProperties.DIGEST_SHA512)
                .setSignaturePaddings(KeyProperties.SIGNATURE_PADDING_RSA_PSS,
                        KeyProperties.SIGNATURE_PADDING_RSA_PKCS1)
                .setCertificateSubject(new X500Principal("CN=UnCall device identity,O=nC"))
                .setCertificateSerialNumber(new BigInteger(160, new SecureRandom()).abs())
                .setCertificateNotBefore(new Date(now - 86_400_000L))
                .setCertificateNotAfter(new Date(now + 315_576_000_000L))
                .setUserAuthenticationRequired(false)
                .build());
        generator.generateKeyPair();
        store.load(null);
        if (store.getCertificate(TLS_KEY_ALIAS) == null) {
            throw new SecurityException("device TLS certificate was not generated");
        }
    }

    private static byte[] aad(Context context, String purpose) {
        return ("UnCall|" + context.getPackageName() + "|" + VAULT_SCHEMA
                + "|" + purpose).getBytes(StandardCharsets.UTF_8);
    }

    private static byte[] legacyAad(Context context, String purpose, String buildId) {
        return ("UnCall|" + context.getPackageName() + "|" + buildId
                + "|" + purpose).getBytes(StandardCharsets.UTF_8);
    }

    private static String decryptWithAad(byte[] iv, byte[] ciphertext, byte[] aad)
            throws Exception {
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE, key(), new GCMParameterSpec(128, iv));
        cipher.updateAAD(aad);
        return new String(cipher.doFinal(ciphertext), StandardCharsets.UTF_8);
    }

    private static String hex(byte[] bytes) {
        StringBuilder out = new StringBuilder(bytes.length * 2);
        for (byte value : bytes) out.append(String.format("%02x", value & 0xff));
        return out.toString();
    }
}
