package dev.nc.blockmaster;

import java.io.ByteArrayOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

final class FrameCodec {
    static final int MAX_FRAME = 1024 * 1024;

    enum Framing {
        LINE,
        FRAME;

        static Framing parse(String value) {
            if ("LINE".equalsIgnoreCase(value)) return LINE;
            if ("FRAME".equalsIgnoreCase(value)) return FRAME;
            throw new IllegalArgumentException("unsupported framing " + value);
        }
    }

    private FrameCodec() {}

    static String readLine(InputStream input) throws IOException {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        while (bytes.size() <= MAX_FRAME) {
            int value = input.read();
            if (value < 0) {
                if (bytes.size() == 0) throw new EOFException("peer closed");
                break;
            }
            if (value == '\n') break;
            if (value != '\r') bytes.write(value);
        }
        if (bytes.size() > MAX_FRAME) throw new IOException("line frame exceeds limit");
        return bytes.toString(StandardCharsets.UTF_8.name());
    }

    static String read(InputStream input, Framing framing) throws IOException {
        if (framing == Framing.LINE) return readLine(input);
        byte[] header = readExact(input, 4);
        int length = ByteBuffer.wrap(header).getInt();
        if (length < 0 || length > MAX_FRAME) throw new IOException("invalid frame length " + length);
        return new String(readExact(input, length), StandardCharsets.UTF_8);
    }

    static void write(OutputStream output, Framing framing, String value) throws IOException {
        byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
        if (bytes.length > MAX_FRAME) throw new IOException("response exceeds frame limit");
        if (framing == Framing.LINE) {
            output.write(bytes);
            output.write('\n');
        } else {
            output.write(ByteBuffer.allocate(4).putInt(bytes.length).array());
            output.write(bytes);
        }
        output.flush();
    }

    private static byte[] readExact(InputStream input, int length) throws IOException {
        byte[] bytes = new byte[length];
        int offset = 0;
        while (offset < length) {
            int read = input.read(bytes, offset, length - offset);
            if (read < 0) throw new EOFException("peer closed inside frame");
            offset += read;
        }
        return bytes;
    }
}
