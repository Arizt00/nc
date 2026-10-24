package dev.nc.blockmaster;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

final class ProtocolSecurity {
    private static final SecureRandom RANDOM = new SecureRandom();

    private ProtocolSecurity() {}

    static String nonce() {
        byte[] value = new byte[24];
        RANDOM.nextBytes(value);
        return hex(value);
    }

    static String hmac(String token, String value) throws Exception {
        Mac mac = Mac.getInstance("HmacSHA256");
        mac.init(new SecretKeySpec(token.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
        return hex(mac.doFinal(value.getBytes(StandardCharsets.UTF_8)));
    }

    static boolean equalHex(String left, String right) {
        if (left == null || right == null || left.length() != 64 || right.length() != 64
                || !left.matches("[0-9a-fA-F]{64}") || !right.matches("[0-9a-fA-F]{64}")) {
            return false;
        }
        return MessageDigest.isEqual(left.toLowerCase().getBytes(StandardCharsets.US_ASCII),
                right.toLowerCase().getBytes(StandardCharsets.US_ASCII));
    }

    static String encode(String value) {
        return Base64.getUrlEncoder().withoutPadding()
                .encodeToString(value.getBytes(StandardCharsets.UTF_8));
    }

    static String decode(String value) {
        return new String(Base64.getUrlDecoder().decode(value), StandardCharsets.UTF_8);
    }

    private static String hex(byte[] bytes) {
        StringBuilder out = new StringBuilder(bytes.length * 2);
        for (byte value : bytes) out.append(String.format("%02x", value & 0xff));
        return out.toString();
    }
}
