package dev.nc.blockmaster;

import java.nio.charset.StandardCharsets;
import java.security.KeyFactory;
import java.security.MessageDigest;
import java.security.PublicKey;
import java.security.Signature;
import java.security.spec.X509EncodedKeySpec;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.Base64;

/**
 * Verifies the immutable UNSP1 data bundled by the nC Android backend and only
 * then materializes it in the nC core. The pinned public key is not a secret;
 * the corresponding development signing authority is deliberately kept out of
 * the generated application and out of the project tree.
 */
final class SignedSpamDatabase {
    private static final String SCHEMA = "UNSP1";
    private static final String PAYLOAD =
            "UNSP1|2026.08.09.1|dev-p256-01|20260809|20270209|4\n"
            + "EXACT|0000000000|100|INVALID_ZERO_ID|BLOCK\n"
            + "EXACT|+0000000000|100|INVALID_ZERO_ID|BLOCK\n"
            + "EXACT|1111111111|90|SPOOF_REPEATED_ID|REVIEW\n"
            + "EXACT|1234567890|85|SPOOF_SEQUENCE_ID|REVIEW";
    private static final String PUBLIC_KEY_BASE64 =
            "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE+UGlRIMmC7hi9/YXNqaaW/FmZlse"
            + "GxL3n32ksFfcmxo7geR0MEQ54GiOO+E4qnb+4lWsR37JfqZkAJ9WCYW/jw==";
    private static final String SIGNATURE_BASE64 =
            "MEQCIEBfSHium2b1cWIGP4OSArUeBxBT+MD3vTECZDN8/N9WAiAukfqVnSl6W4"
            + "HlJ91AbQRZlgD407sHEBRS+3x56CBtIg==";
    private static final String EXPECTED_DIGEST =
            "7C7D9F9C1A775C13A9D7DA6EA1C67FA776F5226B6FFE9A8CC2494660BAD4CF72";

    private SignedSpamDatabase() {}

    static void verifyAndInstall() throws Exception {
        byte[] payload = PAYLOAD.getBytes(StandardCharsets.UTF_8);
        String digest = hex(MessageDigest.getInstance("SHA-256").digest(payload));
        if (!EXPECTED_DIGEST.equals(digest)) {
            throw new SecurityException("UNSP1 digest mismatch");
        }

        PublicKey publicKey = KeyFactory.getInstance("EC").generatePublic(
                new X509EncodedKeySpec(Base64.getDecoder().decode(PUBLIC_KEY_BASE64)));
        Signature verifier = Signature.getInstance("SHA256withECDSA");
        verifier.initVerify(publicKey);
        verifier.update(payload);
        if (!verifier.verify(Base64.getDecoder().decode(SIGNATURE_BASE64))) {
            throw new SecurityException("UNSP1 signature rejected");
        }

        String[] lines = PAYLOAD.split("\\n", -1);
        String[] header = lines[0].split("\\|", -1);
        if (header.length != 6 || !SCHEMA.equals(header[0])) {
            throw new SecurityException("UNSP1 header rejected");
        }
        int issued = parseDate(header[3]);
        int expires = parseDate(header[4]);
        int count = Integer.parseInt(header[5]);
        int today = Integer.parseInt(LocalDate.now().format(DateTimeFormatter.BASIC_ISO_DATE));
        if (today < issued || today > expires || count != lines.length - 1 || count < 1) {
            throw new SecurityException("UNSP1 date or count rejected");
        }

        if (!NativeBridge.beginSignedSpamBase(header[1], header[2], issued, expires, count)) {
            throw new SecurityException("nC rejected UNSP1 header");
        }
        for (int index = 1; index < lines.length; index++) {
            String[] item = lines[index].split("\\|", -1);
            if (item.length != 5 || !NativeBridge.addSignedSpamIndicator(
                    item[0], item[1], Integer.parseInt(item[2]), item[3], item[4])) {
                NativeBridge.invalidateSignedSpamBase("INDICATOR_REJECTED");
                throw new SecurityException("nC rejected UNSP1 indicator");
            }
        }
        if (!NativeBridge.commitSignedSpamBase(digest)) {
            NativeBridge.invalidateSignedSpamBase("COMMIT_REJECTED");
            throw new SecurityException("nC rejected UNSP1 commit");
        }
    }

    private static int parseDate(String value) {
        LocalDate.parse(value, DateTimeFormatter.BASIC_ISO_DATE);
        return Integer.parseInt(value);
    }

    private static String hex(byte[] bytes) {
        StringBuilder value = new StringBuilder(bytes.length * 2);
        for (byte item : bytes) value.append(String.format("%02X", item & 0xff));
        return value.toString();
    }
}
