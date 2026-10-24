package dev.nc.blockmaster;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;
import javax.net.ssl.HttpsURLConnection;
import java.net.URL;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit, privacy-filtered factual web-research adapter for Nuinui. */
final class NuinuiWebSearch {
    private static final int MAX_QUERY = 180;
    private static final int MAX_RESPONSE = 65_536;
    private static final Set<String> WIKIPEDIA_LANGUAGES = new HashSet<>(Arrays.asList(
            "ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "nl",
            "pt", "ru", "tr", "zh"));

    interface Callback {
        void complete(Result result);
    }

    static final class Result {
        final boolean success;
        final String title;
        final String summary;
        final String sourceUrl;
        final String error;

        Result(boolean success, String title, String summary, String sourceUrl, String error) {
            this.success = success;
            this.title = title == null ? "" : title;
            this.summary = summary == null ? "" : summary;
            this.sourceUrl = sourceUrl == null ? "" : sourceUrl;
            this.error = error == null ? "" : error;
        }
    }

    private NuinuiWebSearch() {}

    static void search(String rawQuery, String rawLanguage, Callback callback) {
        String query = rawQuery == null ? "" : rawQuery.trim();
        if (query.isEmpty() || query.length() > MAX_QUERY || containsPhoneNumber(query)) {
            callback.complete(new Result(false, "", "", "", "PRIVATE_OR_INVALID_QUERY"));
            return;
        }
        String language = safeLanguage(rawLanguage);
        new Thread(() -> callback.complete(fetch(query, language)), "nuinui-web-once").start();
    }

    private static Result fetch(String query, String language) {
        HttpsURLConnection connection = null;
        try {
            String encoded = URLEncoder.encode(query, StandardCharsets.UTF_8.name());
            String endpoint = "https://" + language + ".wikipedia.org/w/api.php"
                    + "?action=query&generator=search&gsrnamespace=0&gsrlimit=1"
                    + "&gsrsearch=" + encoded
                    + "&prop=extracts%7Cinfo&exintro=1&explaintext=1&exsentences=4"
                    + "&inprop=url&format=json&formatversion=2&utf8=1";
            URL url = new URL(endpoint);
            connection = (HttpsURLConnection) url.openConnection();
            connection.setRequestMethod("GET");
            connection.setConnectTimeout(7_000);
            connection.setReadTimeout(9_000);
            connection.setInstanceFollowRedirects(false);
            connection.setUseCaches(false);
            connection.setRequestProperty("Accept", "application/json");
            connection.setRequestProperty("Accept-Encoding", "identity");
            connection.setRequestProperty("User-Agent",
                    "UnCall-Nuinui/1.6.0 (com.aristote.uncall; Android)");
            int status = connection.getResponseCode();
            if (status != 200) {
                return new Result(false, "", "", "", "HTTP_" + status);
            }
            String contentType = connection.getContentType();
            if (contentType == null || !contentType.toLowerCase(Locale.ROOT)
                    .contains("application/json")) {
                return new Result(false, "", "", "", "UNEXPECTED_CONTENT");
            }
            String body;
            try (InputStream stream = connection.getInputStream()) {
                body = readLimited(stream);
            }
            JSONObject queryObject = new JSONObject(body).optJSONObject("query");
            JSONArray pages = queryObject == null ? null : queryObject.optJSONArray("pages");
            if (pages == null || pages.length() == 0) {
                return new Result(false, "", "", "", "NO_RESULT");
            }
            JSONObject page = pages.optJSONObject(0);
            if (page == null) return new Result(false, "", "", "", "NO_RESULT");
            String title = page.optString("title", "").trim();
            String extract = page.optString("extract", "").trim();
            String source = page.optString("fullurl", "").trim();
            if (title.isEmpty() || extract.isEmpty() || !source.startsWith("https://")) {
                return new Result(false, "", "", "", "INCOMPLETE_RESULT");
            }
            return new Result(true, title, extract, source, "");
        } catch (Exception error) {
            return new Result(false, "", "", "", "NETWORK_ERROR");
        } finally {
            if (connection != null) connection.disconnect();
        }
    }

    private static String readLimited(InputStream stream) throws Exception {
        StringBuilder body = new StringBuilder();
        char[] buffer = new char[2_048];
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(
                stream, StandardCharsets.UTF_8))) {
            int read;
            while ((read = reader.read(buffer)) >= 0) {
                if (body.length() + read > MAX_RESPONSE) {
                    throw new IllegalStateException("response too large");
                }
                body.append(buffer, 0, read);
            }
        }
        return body.toString();
    }

    private static boolean containsPhoneNumber(String query) {
        int consecutiveDigits = 0;
        int totalDigits = 0;
        for (int index = 0; index < query.length(); index++) {
            char value = query.charAt(index);
            if (Character.isDigit(value)) {
                consecutiveDigits++;
                totalDigits++;
                if (consecutiveDigits >= 7 || totalDigits >= 9) return true;
            } else if (value == ' ' || value == '-' || value == '(' || value == ')') {
                // Formatting inside a possible phone number does not reset totalDigits.
                consecutiveDigits = 0;
            } else {
                consecutiveDigits = 0;
                totalDigits = 0;
            }
        }
        return false;
    }

    private static String safeLanguage(String rawLanguage) {
        String language = rawLanguage == null ? "en"
                : rawLanguage.toLowerCase(Locale.ROOT).trim();
        return WIKIPEDIA_LANGUAGES.contains(language) ? language : "en";
    }
}
