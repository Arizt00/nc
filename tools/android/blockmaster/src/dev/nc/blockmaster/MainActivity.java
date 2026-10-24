package dev.nc.blockmaster;

import android.Manifest;
import android.app.Activity;
import android.app.Dialog;
import android.app.role.RoleManager;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.res.ColorStateList;
import android.content.res.Configuration;
import android.graphics.Color;
import android.graphics.PorterDuff;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.graphics.drawable.StateListDrawable;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.PersistableBundle;
import android.os.SystemClock;
import android.provider.Settings;
import android.text.InputType;
import android.text.InputFilter;
import android.text.format.DateUtils;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.view.accessibility.AccessibilityManager;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.animation.DecelerateInterpolator;
import android.view.animation.OvershootInterpolator;
import android.view.animation.PathInterpolator;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;
import android.widget.Toast;
import java.io.IOException;
import java.text.DateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

public final class MainActivity extends Activity {
    private static final int ROLE_REQUEST = 1201;
    private static final int PERMISSIONS_REQUEST = 1202;
    private static final int CONTACTS_REQUEST = 1203;
    private static final int TAB_HOME = 0;
    private static final int TAB_ACTIVITY = 1;
    private static final int TAB_RULES = 2;
    private static final int TAB_SETTINGS = 3;
    private static final String PREF_THEME_MODE = "ui_theme_mode";
    private static final String PREF_LANGUAGE_MODE = "ui_language_mode";
    private static final String PREF_AI_CONSENT = "adaptive_ai_explicit_consent";
    private static final String THEME_SYSTEM = "system";
    private static final String THEME_LIGHT = "light";
    private static final String THEME_DARK = "dark";
    private static final String LANGUAGE_SYSTEM = "system";
    private static final String LANGUAGE_ES = "es";
    private static final String LANGUAGE_EN = "en";
    private static final String STATE_TAB = "uncall.current.tab";

    private FrameLayout pageHost;
    private LinearLayout bottomBar;
    private TextView settingsButton;
    private View topGuard;
    private final ImageView[] navItems = new ImageView[3];
    private final int[] scrollPositions = new int[4];
    private final Object mutationLock = new Object();
    private final Set<String> expandedActivityDays = new HashSet<>();
    private final Set<String> expandedActivityGroups = new HashSet<>();
    private ScrollView currentScroll;
    private SharedPreferences prefs;
    private int currentTab = TAB_HOME;
    private int lastContentTab = TAB_HOME;
    private int pageTransitionGeneration;
    private boolean dark;
    private int colorBackground;
    private int colorSurface;
    private int colorText;
    private int colorMuted;
    private int colorBorder;
    private int colorGreen;
    private int colorGreenStrong;
    private int colorGreenSoft;
    private int colorRed;
    private int colorRedSoft;
    private int lastScrollY;
    private boolean navigationHidden;
    private boolean navigationSuppressed;
    private boolean activityExpansionInitialized;
    private long lastActivityRefresh;
    private AccessibilityManager accessibilityManager;
    private float systemFontScale = 1f;
    private boolean highContrast;
    private boolean reducedMotion;
    private boolean compactContent;
    private boolean tabletContent;

    private final SharedPreferences.OnSharedPreferenceChangeListener preferenceListener =
            (preferences, key) -> {
                if (BlockMasterService.PREF_LAST_DECISION.equals(key)
                        || BlockMasterService.PREF_LAST_DECISION_AT.equals(key)) {
                    runOnUiThread(this::refreshCurrentPage);
                }
            };

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        prefs = getSharedPreferences(BlockMasterService.PREFS, MODE_PRIVATE);
        readAccessibilityState();
        applyLanguageDirection();
        configurePalette();
        configureWindow();
        try {
            SecurityVault.assertTrustedSigner(this);
            if (BuildConfig.REMOTE_CONTROL_ENABLED) ensureToken();
        } catch (Exception securityFailure) {
            showSecurityFailure();
            return;
        }
        currentTab = state == null ? TAB_HOME : state.getInt(STATE_TAB, TAB_HOME);
        setContentView(buildShell());
        showTab(currentTab);
        requestMissingPermissions();
        if (BuildConfig.REMOTE_CONTROL_ENABLED) {
            startForegroundService(new Intent(this, BlockMasterService.class));
        }
    }

    @Override protected void onSaveInstanceState(Bundle state) {
        captureScrollPosition();
        state.putInt(STATE_TAB, currentTab);
        super.onSaveInstanceState(state);
    }

    @Override protected void onResume() {
        super.onResume();
        prefs.registerOnSharedPreferenceChangeListener(preferenceListener);
        boolean previousContrast = highContrast;
        boolean previousMotion = reducedMotion;
        boolean previousCompact = compactContent;
        float previousFontScale = systemFontScale;
        readAccessibilityState();
        if (pageHost != null && (previousContrast != highContrast
                || previousMotion != reducedMotion
                || previousCompact != compactContent
                || Math.abs(previousFontScale - systemFontScale) > 0.01f)) {
            rebuildShellKeepingPage();
            return;
        }
        refreshCurrentPage();
    }

    @Override protected void onPause() {
        prefs.unregisterOnSharedPreferenceChangeListener(preferenceListener);
        super.onPause();
    }

    @Override public void onBackPressed() {
        if (currentTab == TAB_SETTINGS) {
            showTab(lastContentTab);
            return;
        }
        super.onBackPressed();
    }

    private void configurePalette() {
        String themeMode = prefs.getString(PREF_THEME_MODE, THEME_SYSTEM);
        dark = THEME_DARK.equals(themeMode) || (THEME_SYSTEM.equals(themeMode)
                && (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                == Configuration.UI_MODE_NIGHT_YES);
        if (dark) {
            colorBackground = Color.rgb(14, 19, 17);
            colorSurface = Color.rgb(23, 30, 27);
            colorText = Color.rgb(241, 245, 243);
            colorMuted = Color.rgb(163, 174, 169);
            colorBorder = Color.argb(38, 255, 255, 255);
            colorGreenSoft = Color.rgb(20, 57, 39);
            colorRedSoft = Color.rgb(67, 31, 32);
        } else {
            colorBackground = Color.rgb(247, 249, 248);
            colorSurface = Color.WHITE;
            colorText = Color.rgb(17, 24, 39);
            colorMuted = Color.rgb(102, 112, 126);
            colorBorder = Color.argb(42, 15, 156, 73);
            colorGreenSoft = Color.rgb(236, 252, 243);
            colorRedSoft = Color.rgb(255, 241, 241);
        }
        colorGreen = Color.rgb(34, 197, 94);
        colorGreenStrong = Color.rgb(15, 156, 73);
        colorRed = Color.rgb(244, 70, 70);
        if (highContrast) {
            colorText = dark ? Color.WHITE : Color.BLACK;
            colorMuted = dark ? Color.rgb(221, 229, 225) : Color.rgb(47, 57, 69);
            colorBorder = dark ? Color.argb(150, 255, 255, 255)
                    : Color.argb(145, 15, 120, 60);
        }
    }

    private void readAccessibilityState() {
        Configuration configuration = getResources().getConfiguration();
        systemFontScale = Math.max(1f, configuration.fontScale);
        int widthDp = configuration.screenWidthDp > 0
                ? configuration.screenWidthDp
                : Math.round(getResources().getDisplayMetrics().widthPixels
                        / getResources().getDisplayMetrics().density);
        compactContent = widthDp < 330 || systemFontScale >= 1.30f;
        tabletContent = widthDp >= 600;
        accessibilityManager = (AccessibilityManager) getSystemService(
                Context.ACCESSIBILITY_SERVICE);
        try {
            highContrast = Settings.Secure.getInt(getContentResolver(),
                    "high_text_contrast_enabled", 0) == 1;
        } catch (Exception ignored) {
            highContrast = false;
        }
        float animatorScale = 1f;
        try {
            animatorScale = Settings.Global.getFloat(getContentResolver(),
                    Settings.Global.ANIMATOR_DURATION_SCALE, 1f);
        } catch (Exception ignored) {}
        reducedMotion = animatorScale <= 0.01f;
    }

    private void configureWindow() {
        Window window = getWindow();
        window.clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS
                | WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION);
        window.setStatusBarColor(colorBackground);
        window.setNavigationBarColor(colorBackground);
        if (Build.VERSION.SDK_INT >= 29) {
            window.setStatusBarContrastEnforced(false);
            window.setNavigationBarContrastEnforced(false);
        }
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(true);
            window.setStatusBarColor(colorBackground);
            window.setNavigationBarColor(colorBackground);
            window.getDecorView().getWindowInsetsController().setSystemBarsAppearance(
                    dark ? 0 : android.view.WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                            | android.view.WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS,
                    android.view.WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                            | android.view.WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS);
        } else if (Build.VERSION.SDK_INT >= 23) {
            window.getDecorView().setSystemUiVisibility(dark ? 0
                    : View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        }
    }

    private View buildShell() {
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(colorBackground);

        pageHost = new FrameLayout(this);
        root.addView(pageHost, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

        topGuard = new View(this);
        topGuard.setBackground(topGuardBackground());
        topGuard.setAlpha(0f);
        topGuard.setVisibility(View.INVISIBLE);
        topGuard.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        FrameLayout.LayoutParams guardParams = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(112), Gravity.TOP);
        root.addView(topGuard, guardParams);

        bottomBar = new LinearLayout(this);
        bottomBar.setOrientation(LinearLayout.HORIZONTAL);
        bottomBar.setGravity(Gravity.CENTER);
        bottomBar.setPadding(dp(5), dp(5), dp(5), dp(5));
        bottomBar.setBackground(glass(28, true));
        bottomBar.setElevation(0f);
        String[] labels = {l("Inicio", "Home"), l("Actividad", "Activity"),
                l("Reglas", "Rules")};
        String[] icons = {"nav_home", "nav_activity", "nav_rules"};
        for (int i = 0; i < labels.length; i++) {
            final int tab = i;
            FrameLayout slot = new FrameLayout(this);
            ImageView item = new ImageView(this);
            item.setScaleType(ImageView.ScaleType.CENTER_INSIDE);
            item.setPadding(dp(14), dp(14), dp(14), dp(14));
            int iconId = getResources().getIdentifier(icons[i], "drawable", getPackageName());
            if (iconId != 0) item.setImageResource(iconId);
            item.setColorFilter(colorMuted, PorterDuff.Mode.SRC_IN);
            item.setClickable(true);
            item.setFocusable(true);
            item.setContentDescription(labels[i]);
            exposeAsButton(item);
            item.setOnClickListener(view -> showTab(tab));
            installNavPressFeedback(item);
            navItems[i] = item;
            FrameLayout.LayoutParams iconParams = new FrameLayout.LayoutParams(dp(52), dp(52));
            iconParams.gravity = Gravity.CENTER;
            slot.addView(item, iconParams);
            bottomBar.addView(slot, new LinearLayout.LayoutParams(0, dp(58), 1f));
        }
        FrameLayout.LayoutParams barParams = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        barParams.gravity = Gravity.BOTTOM;
        // Preserve the reserved lower breathing room while making the capsule
        // visibly narrower on phones, as requested by the nC UI contract.
        barParams.setMargins(dp(44), 0, dp(44), dp(16));
        root.addView(bottomBar, barParams);

        settingsButton = text("•••", 16, colorText, Typeface.BOLD);
        settingsButton.setGravity(Gravity.CENTER);
        settingsButton.setLetterSpacing(0.04f);
        settingsButton.setContentDescription(l("Abrir ajustes", "Open settings"));
        settingsButton.setBackground(settingsMenuBackground(false));
        settingsButton.setForeground(settingsMenuRipple());
        settingsButton.setClickable(true);
        settingsButton.setFocusable(true);
        settingsButton.setMinWidth(dp(48));
        settingsButton.setMinHeight(dp(48));
        exposeAsButton(settingsButton);
        settingsButton.setOnClickListener(view -> {
            if (currentTab == TAB_SETTINGS) showTab(lastContentTab);
            else showTab(TAB_SETTINGS);
        });
        installMenuPressFeedback(settingsButton);
        FrameLayout.LayoutParams settingsParams = new FrameLayout.LayoutParams(dp(48), dp(48));
        settingsParams.gravity = Gravity.TOP | Gravity.END;
        settingsParams.setMargins(0, dp(36), dp(18), 0);
        root.addView(settingsButton, settingsParams);
        return root;
    }

    private void showTab(int tab) {
        captureScrollPosition();
        int oldTab = currentTab;
        if (tab == TAB_SETTINGS && oldTab != TAB_SETTINGS) lastContentTab = oldTab;
        else if (tab != TAB_SETTINGS) lastContentTab = tab;
        currentTab = tab;
        View page;
        if (tab == TAB_ACTIVITY) page = buildActivityPage();
        else if (tab == TAB_RULES) page = buildRulesPage();
        else if (tab == TAB_SETTINGS) page = buildSettingsPage();
        else page = buildHomePage();
        currentScroll = page instanceof ScrollView ? (ScrollView) page : null;
        int generation = ++pageTransitionGeneration;
        boolean animate = pageHost.getChildCount() > 0 && oldTab != tab;
        if (animate) {
            transitionToPage(page, oldTab, tab, generation);
        } else {
            attachPage(page, tab, generation);
        }
        updateNavigationForTab(tab);
        for (int i = 0; i < navItems.length; i++) {
            boolean selected = i == tab;
            ImageView item = navItems[i];
            item.animate().cancel();
            item.setColorFilter(selected ? colorGreenStrong : colorMuted,
                    PorterDuff.Mode.SRC_IN);
            item.setBackground(navBackground(false));
            item.setSelected(selected);
            if (Build.VERSION.SDK_INT >= 30) item.setStateDescription(null);
            item.animate()
                    .scaleX(selected ? 1.22f : 0.82f)
                    .scaleY(selected ? 1.22f : 0.82f)
                    .alpha(selected ? 1f : 0.70f)
                    .setInterpolator(selected
                            ? new OvershootInterpolator(1.18f)
                            : new DecelerateInterpolator(1.6f))
                    .setDuration(motionDuration(selected ? 320L : 220L))
                    .start();
        }
        updateSettingsButton(tab == TAB_SETTINGS);
        if (oldTab != tab) {
            String announcement = tabTitle(tab);
            page.post(() -> page.announceForAccessibility(announcement));
        }
    }

    private String tabTitle(int tab) {
        if (tab == TAB_ACTIVITY) return l("Actividad", "Activity");
        if (tab == TAB_RULES) return l("Reglas", "Rules");
        if (tab == TAB_SETTINGS) return l("Ajustes", "Settings");
        return l("Inicio", "Home");
    }

    private void attachPage(View page, int tab, int generation) {
        if (generation != pageTransitionGeneration) return;
        pageHost.removeAllViews();
        page.setAlpha(1f);
        page.setTranslationX(0f);
        page.setTranslationY(0f);
        page.setScaleX(1f);
        page.setScaleY(1f);
        pageHost.addView(page, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        restoreScrollPosition(tab);
    }

    private void transitionToPage(View page, int oldTab, int tab, int generation) {
        if (generation != pageTransitionGeneration || pageHost.getChildCount() == 0) return;
        View outgoing = pageHost.getChildAt(pageHost.getChildCount() - 1);
        for (int i = pageHost.getChildCount() - 2; i >= 0; i--) {
            View stale = pageHost.getChildAt(i);
            stale.animate().cancel();
            pageHost.removeViewAt(i);
        }
        outgoing.animate().cancel();
        pageHost.addView(page, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        restoreScrollPosition(tab);

        boolean settingsMotion = oldTab == TAB_SETTINGS || tab == TAB_SETTINGS;
        float direction = tab > oldTab ? 1f : -1f;
        PathInterpolator natural = new PathInterpolator(0.20f, 0f, 0f, 1f);
        page.setAlpha(reducedMotion ? 0f : (settingsMotion ? 0f : 0.28f));
        page.setTranslationX(reducedMotion ? 0f : (settingsMotion ? 0f : direction * dp(42)));
        page.setTranslationY(reducedMotion ? 0f : (settingsMotion ? dp(24) : 0f));
        page.setScaleX(reducedMotion ? 1f : (settingsMotion ? 0.985f : 0.995f));
        page.setScaleY(reducedMotion ? 1f : (settingsMotion ? 0.985f : 0.995f));

        outgoing.animate()
                .alpha(0f)
                .translationX(reducedMotion ? 0f : (settingsMotion ? 0f : -direction * dp(28)))
                .translationY(reducedMotion ? 0f : (settingsMotion ? -dp(10) : 0f))
                .scaleX(reducedMotion ? 1f : (settingsMotion ? 0.99f : 0.995f))
                .scaleY(reducedMotion ? 1f : (settingsMotion ? 0.99f : 0.995f))
                .setInterpolator(natural)
                .setDuration(motionDuration(reducedMotion ? 90L : 250L))
                .withEndAction(() -> {
                    if (generation == pageTransitionGeneration && outgoing.getParent() == pageHost) {
                        pageHost.removeView(outgoing);
                    }
                })
                .start();

        page.animate()
                .alpha(1f)
                .translationX(0f)
                .translationY(0f)
                .scaleX(1f)
                .scaleY(1f)
                .setInterpolator(natural)
                .setDuration(motionDuration(reducedMotion ? 120L
                        : (settingsMotion ? 380L : 340L)))
                .withEndAction(() -> {
                    if (generation != pageTransitionGeneration) return;
                    for (int i = pageHost.getChildCount() - 1; i >= 0; i--) {
                        View child = pageHost.getChildAt(i);
                        if (child != page) pageHost.removeViewAt(i);
                    }
                })
                .start();
    }

    private void restoreScrollPosition(int tab) {
        if (currentScroll != null) {
            int position = scrollPositions[Math.max(0, Math.min(tab, scrollPositions.length - 1))];
            ScrollView pageScroll = currentScroll;
            pageScroll.post(() -> pageScroll.scrollTo(0, position));
        }
    }

    private void updateSettingsButton(boolean active) {
        if (settingsButton == null) return;
        settingsButton.animate().cancel();
        settingsButton.setTextColor(active ? colorGreenStrong : colorText);
        settingsButton.setBackground(settingsMenuBackground(active));
        settingsButton.setForeground(settingsMenuRipple());
        settingsButton.setContentDescription(active
                ? l("Cerrar ajustes", "Close settings")
                : l("Abrir ajustes", "Open settings"));
        updateTopGuard(active && currentScroll != null ? currentScroll.getScrollY() : 0);
        settingsButton.animate()
                .rotation(active ? 90f : 0f)
                .setInterpolator(new DecelerateInterpolator())
                .setDuration(motionDuration(260L))
                .start();
    }

    private void refreshCurrentPage() {
        if (pageHost == null) return;
        showTab(currentTab);
    }

    private void captureScrollPosition() {
        if (currentScroll == null || currentTab < 0 || currentTab >= scrollPositions.length) return;
        scrollPositions[currentTab] = currentScroll.getScrollY();
    }

    private void rebuildShellKeepingPage() {
        captureScrollPosition();
        int tab = currentTab;
        configurePalette();
        configureWindow();
        setContentView(buildShell());
        showTab(tab);
    }

    private View buildHomePage() {
        LinearLayout page = pageBody();
        page.addView(brandHeader(l("Tus llamadas, bajo control.",
                "Your calls, under control.")));

        String status = safeCommand("ESTADO");
        boolean protection = !"PAUSED".equals(field(status, "PROTECTION", "ACTIVE"));
        boolean roleHeld = hasScreeningRole();

        LinearLayout hero = card();
        TextView eyebrow = text(l("FILTRO DE LLAMADAS", "CALL FILTER"),
                12, colorMuted, Typeface.BOLD);
        hero.addView(eyebrow);
        TextView active = text(roleHeld && protection ? l("ACTIVO", "ACTIVE")
                        : l("REQUIERE ATENCIÓN", "NEEDS ATTENTION"),
                roleHeld && protection ? 38 : 27,
                roleHeld && protection ? colorGreen : colorRed, Typeface.BOLD);
        active.setPadding(0, dp(8), 0, dp(4));
        hero.addView(active);
        hero.addView(text(roleHeld && protection
                        ? l("Protegiendo tus llamadas en tiempo real",
                                "Protecting your calls in real time")
                        : !roleHeld ? l("Concede el rol de filtrado para empezar",
                                "Grant the screening role to begin")
                                : l("La protección nC está pausada", "nC protection is paused"),
                16, colorMuted, Typeface.NORMAL));
        TextView rolePill = pill(roleHeld ? l("Rol de filtrado activo", "Screening role active")
                        : l("Rol pendiente", "Role pending"),
                roleHeld ? colorGreenSoft : colorRedSoft,
                roleHeld ? colorGreenStrong : colorRed);
        LinearLayout.LayoutParams pillParams = wrapParams();
        pillParams.topMargin = dp(18);
        hero.addView(rolePill, pillParams);
        if (!roleHeld) {
            TextView grant = actionButton(l("Activar protección", "Activate protection"),
                    true, view -> requestScreeningRole());
            addWithTop(hero, grant, 14);
        }
        addCard(page, hero);

        LinearLayout stats = new LinearLayout(this);
        stats.setOrientation(compactContent ? LinearLayout.VERTICAL : LinearLayout.HORIZONTAL);
        View blocked = statCard(l("Bloqueadas", "Blocked"),
                field(status, "BLOCKED", "0"), colorRed, colorRedSoft);
        View allowed = statCard(l("Permitidas", "Allowed"),
                field(status, "ALLOWED", "0"), colorGreen, colorGreenSoft);
        LinearLayout.LayoutParams left = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        left.rightMargin = dp(6);
        LinearLayout.LayoutParams right = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        right.leftMargin = dp(6);
        if (compactContent) {
            left = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT);
            left.bottomMargin = dp(12);
            right = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT);
        }
        stats.addView(blocked, left);
        stats.addView(allowed, right);
        addWithBottom(page, stats, 14);

        page.addView(sectionLabel(l("Última decisión", "Last decision")));
        addCard(page, decisionCard(lastDecisionRaw(), lastDecisionAt()));

        page.addView(sectionLabel(l("Protecciones principales", "Main protections")));
        LinearLayout controls = card();
        if (BuildConfig.NATIVE_BMUI1) {
            controls.addView(switchSetting(l("Protección nC", "nC protection"),
                    l("Aplicar todas las reglas activas", "Apply all active rules"),
                    protection, (button, enabled) -> runSwitchMutation(button, enabled,
                            "MODO_PROTECCION " + (enabled ? "ON" : "OFF"),
                            enabled ? l("Protección activada", "Protection enabled")
                                    : l("Protección pausada", "Protection paused"), null)));
            controls.addView(divider());
        }
        boolean contactsPermission = checkSelfPermission(Manifest.permission.READ_CONTACTS)
                == PackageManager.PERMISSION_GRANTED;
        boolean contactsOnly = contactsPermission && "ON".equals(field(status, "CONTACTS_ONLY",
                prefs.getBoolean(BlockMasterService.PREF_CONTACTS_ONLY, true) ? "ON" : "OFF"));
        controls.addView(switchSetting(l("Solo contactos", "Contacts only"),
                l("Bloquear números que no están guardados",
                        "Block numbers that are not saved"), contactsOnly, (button, enabled) -> {
                    if (enabled && BuildConfig.NATIVE_CONTACTS
                            && checkSelfPermission(Manifest.permission.READ_CONTACTS)
                            != PackageManager.PERMISSION_GRANTED) {
                        button.setChecked(false);
                        toast(l("Concede acceso a contactos para activar esta protección",
                                "Grant contacts access to enable this protection"));
                        reviewContactsPermission();
                        return;
                    }
                    runSwitchMutation(button, enabled,
                            "MODO_CONTACTOS " + (enabled ? "ON" : "OFF"),
                            enabled ? l("Solo contactos activado", "Contacts-only enabled")
                                    : l("Solo contactos desactivado", "Contacts-only disabled"),
                            () -> prefs.edit().putBoolean(
                                    BlockMasterService.PREF_CONTACTS_ONLY, enabled).apply());
                }));
        if (BuildConfig.NATIVE_BMUI1) {
            controls.addView(divider());
            boolean hidden = "ON".equals(field(status, "BLOCK_HIDDEN", "ON"));
            controls.addView(switchSetting(l("Números ocultos", "Hidden numbers"),
                    l("Rechazar llamadas sin identificador", "Reject calls without caller ID"),
                    hidden, (button, enabled) ->
                            runSwitchMutation(button, enabled,
                                    "BLOQUEAR_OCULTOS " + (enabled ? "ON" : "OFF"),
                                    enabled ? l("Números ocultos bloqueados", "Hidden numbers blocked")
                                            : l("Números ocultos permitidos", "Hidden numbers allowed"),
                                    null)));
        }
        addCard(page, controls);

        page.addView(sectionLabel(l("Servicio y conexión", "Service and connection")));
        LinearLayout connection = card();
        connection.addView(labelValue(l("Servicio", "Service"),
                l("Filtrado local activo", "Local screening active"), colorGreenStrong));
        if (BuildConfig.REMOTE_CONTROL_ENABLED) {
            connection.addView(divider());
            connection.addView(labelValue(l("Conexión segura", "Secure connection"),
                    l("Identidad por dispositivo · TLS 8443",
                            "Per-device identity · TLS 8443"), colorGreenStrong));
            if (BuildConfig.LEGACY_USB_CONTROL_ENABLED) {
                connection.addView(divider());
                connection.addView(labelValue(l("Control de desarrollo", "Development control"),
                        l("Solo USB/localhost · 8080", "USB/localhost only · 8080"), colorMuted));
            }
        } else {
            connection.addView(divider());
            connection.addView(labelValue(l("Privacidad", "Privacy"),
                    l("Sin servidor ni puertos abiertos", "No server or open ports"),
                    colorGreenStrong));
        }
        addCard(page, connection);
        return scroll(page);
    }

    private View buildActivityPage() {
        LinearLayout page = pageBody();
        page.addView(pageHeader(l("Actividad", "Activity"),
                l("Cada decisión, con su regla y motivo.",
                        "Every decision, with its rule and reason.")));
        addCard(page, decisionCard(lastDecisionRaw(), lastDecisionAt()));

        LinearLayout historyHeading = new LinearLayout(this);
        historyHeading.setOrientation(LinearLayout.HORIZONTAL);
        historyHeading.setGravity(Gravity.CENTER_VERTICAL);
        historyHeading.addView(sectionLabel(l("Historial reciente", "Recent history")),
                new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        if (lastActivityRefresh > 0L) {
            historyHeading.addView(pill(l("Actualizado ", "Updated ")
                            + android.text.format.DateFormat.format("HH:mm:ss", lastActivityRefresh),
                    colorGreenSoft, colorGreenStrong));
        }
        page.addView(historyHeading);
        LinearLayout historyCard = card();
        List<DecisionEvent> events = BuildConfig.NATIVE_BMUI1
                ? loadActivityEvents() : new ArrayList<>();
        if (events.isEmpty()) {
            historyCard.addView(emptyState(
                    l("Todavía no hay actividad registrada.", "No activity has been recorded yet."),
                    l("Las llamadas evaluadas aparecerán aquí.",
                            "Evaluated calls will appear here.")));
        } else {
            addGroupedActivity(historyCard, events);
        }
        addCard(page, historyCard);
        TextView refresh = actionButton(l("Actualizar actividad", "Refresh activity"),
                false, view -> {
                    lastActivityRefresh = System.currentTimeMillis();
                    refreshCurrentPage();
                    toast(l("Actividad actualizada", "Activity refreshed"));
                });
        addWithBottom(page, refresh, 20);
        return scroll(page);
    }

    private View buildRulesPage() {
        LinearLayout page = pageBody();
        page.addView(pageHeader(l("Reglas", "Rules"),
                l("Gestiona bloqueos y excepciones desde el móvil.",
                        "Manage blocks and exceptions from your phone.")));
        RuleGroups groups = parseRules(safeCommand("LISTAR"));
        addRuleSection(page, "BLOCKLIST", l("Números bloqueados", "Blocked numbers"),
                l("Coincidencia exacta", "Exact match"), groups.blocked,
                "AGREGAR ", "ELIMINAR ", l("Número a bloquear", "Number to block"), false);
        addRuleSection(page, "BLOCK_PATTERNS", l("Patrones bloqueados", "Blocked patterns"),
                l("Bloqueo por coincidencia contenida", "Block when text contains pattern"),
                groups.blockPatterns, "BLOQUEAR_POR_PATRON ", "DESBLOQUEAR_PATRON ",
                l("Patrón a bloquear", "Pattern to block"), true);
        addRuleSection(page, "WHITELIST", l("Números permitidos", "Allowed numbers"),
                l("Excepciones exactas", "Exact exceptions"),
                groups.allowed, "AGREGAR_WHITELIST ", "ELIMINAR_WHITELIST ",
                l("Número a permitir", "Number to allow"), false);
        addRuleSection(page, "ALLOW_PATTERNS", l("Patrones permitidos", "Allowed patterns"),
                l("Excepciones para prefijos o grupos", "Exceptions for prefixes or groups"),
                groups.allowPatterns,
                BuildConfig.NATIVE_BMUI1 ? "PERMITIR_POR_PATRON " : "AGREGAR_WHITELIST PATRON::",
                BuildConfig.NATIVE_BMUI1 ? "QUITAR_PERMISO_PATRON " : "ELIMINAR_WHITELIST PATRON::",
                l("Patrón a permitir", "Pattern to allow"), true);
        return scroll(page);
    }

    private View buildSettingsPage() {
        LinearLayout page = pageBody();
        page.addView(pageHeader(l("Ajustes", "Settings"),
                l("Permisos, apariencia, seguridad y mantenimiento.",
                        "Permissions, appearance, security and maintenance.")));

        page.addView(sectionLabel(l("Apariencia e idioma", "Appearance and language")));
        LinearLayout appearance = card();
        appearance.addView(choiceSetting(l("Tema", "Theme"),
                l("Elige cómo se muestra UnCall", "Choose how UnCall looks"),
                new String[] {l("Sistema", "System"), l("Claro", "Light"), l("Oscuro", "Dark")},
                new String[] {THEME_SYSTEM, THEME_LIGHT, THEME_DARK},
                prefs.getString(PREF_THEME_MODE, THEME_SYSTEM), value -> {
                    prefs.edit().putString(PREF_THEME_MODE, value).commit();
                    rebuildShellKeepingPage();
                }));
        appearance.addView(divider());
        String languageCode = prefs.getString(PREF_LANGUAGE_MODE, LANGUAGE_SYSTEM);
        appearance.addView(actionRow(l("Idioma", "Language"),
                l("Idioma de toda la interfaz", "Language for the whole interface")
                        + " · " + Translations.languageName(languageCode),
                l("Cambiar", "Change"), view -> showLanguageDialog()));
        addCard(page, appearance);

        page.addView(sectionLabel(l("Acceso del sistema", "System access")));
        LinearLayout access = card();
        boolean roleHeld = hasScreeningRole();
        access.addView(actionRow(l("Rol de filtrado", "Call screening role"),
                roleHeld ? l("Activo", "Active") : l("Pendiente", "Pending"),
                roleHeld ? l("Revisar", "Review") : l("Conceder", "Grant"),
                view -> reviewScreeningRole()));
        if (BuildConfig.NATIVE_CONTACTS) {
            access.addView(divider());
            boolean contacts = checkSelfPermission(Manifest.permission.READ_CONTACTS)
                    == PackageManager.PERMISSION_GRANTED;
            access.addView(actionRow(l("Contactos", "Contacts"),
                    contacts ? l("Acceso activo", "Access active")
                            : l("Permiso pendiente", "Permission pending"),
                    contacts ? l("Revisar", "Review") : l("Conceder", "Grant"),
                    view -> reviewContactsPermission()));
        }
        addCard(page, access);

        page.addView(sectionLabel(l("Servicio", "Service")));
        LinearLayout service = card();
        if (BuildConfig.REMOTE_CONTROL_ENABLED) {
            service.addView(actionRow(l("Motor UnCall", "UnCall engine"),
                    l("CallScreeningService y control TLS bajo consentimiento",
                            "CallScreeningService and consent-based TLS control"),
                    l("Iniciar", "Start"), view -> {
                        startForegroundService(new Intent(this, BlockMasterService.class));
                        toast(l("Servicio iniciado", "Service started"));
                    }));
            service.addView(divider());
            service.addView(actionRow(l("Detener control remoto", "Stop remote control"),
                    l("El filtrado local conserva reglas", "Local screening keeps its rules"),
                    l("Detener", "Stop"), view -> {
                        Intent intent = new Intent(this, BlockMasterService.class)
                                .setAction(BlockMasterService.ACTION_STOP);
                        startService(intent);
                        toast(l("Control remoto detenido", "Remote control stopped"));
                    }));
        } else {
            service.addView(labelValue(l("Motor local UnCall", "Local UnCall engine"),
                    l("CallScreeningService · sin proceso permanente",
                            "CallScreeningService · no persistent process"), colorGreenStrong));
            service.addView(divider());
            service.addView(labelValue(l("Red", "Network"),
                    l("Desactivada en la edición comercial",
                            "Disabled in the store edition"), colorGreenStrong));
        }
        addCard(page, service);

        if (BuildConfig.REMOTE_CONTROL_ENABLED) {
            page.addView(sectionLabel(l("Conexión segura", "Secure connection")));
            LinearLayout secure = card();
            String token;
            String fingerprint;
            try {
                token = SecurityVault.getOrCreateAccessToken(this, prefs);
                fingerprint = SecurityVault.tlsCertificateSha256();
            } catch (Exception unavailable) {
                token = "";
                fingerprint = "";
            }
            secure.addView(labelValue(l("TLS autenticado", "Authenticated TLS"),
                    l("Clave privada no exportable · 8443",
                            "Non-exportable private key · 8443"), colorGreenStrong));
            secure.addView(divider());
            secure.addView(labelValue(l("Huella del dispositivo", "Device fingerprint"),
                    maskToken(fingerprint), colorMuted));
            secure.addView(divider());
            secure.addView(actionRow(l("Token de desarrollo", "Development token"),
                    maskToken(token), l("Copiar", "Copy"), view -> copyToken()));
            addCard(page, secure);
        }

        page.addView(sectionLabel(l("Seguridad reforzada", "Hardened security")));
        LinearLayout hardened = card();
        hardened.addView(labelValue(l("Protección criptográfica", "Cryptographic protection"),
                l("AES-256-GCM · Android Keystore", "AES-256-GCM · Android Keystore"),
                colorGreenStrong));
        hardened.addView(divider());
        hardened.addView(labelValue(l("Integridad de aplicación", "Application integrity"),
                l("Firma verificada", "Verified signer"), colorGreenStrong));
        hardened.addView(divider());
        hardened.addView(labelValue(l("Superficie de red", "Network surface"),
                BuildConfig.REMOTE_CONTROL_ENABLED
                        ? l("Control explícito con identidad por dispositivo",
                                "Explicit control with per-device identity")
                        : l("Sin permisos de red en la edición comercial",
                                "No network permissions in the store edition"),
                colorGreenStrong));
        addCard(page, hardened);

        page.addView(sectionLabel(l("Privacidad y datos", "Privacy and data")));
        LinearLayout privacy = card();
        privacy.addView(stackedLabelValue(l("Tratamiento", "Processing"),
                l("Contactos y decisiones permanecen en el dispositivo",
                        "Contacts and decisions remain on this device"), colorGreenStrong));
        privacy.addView(divider());
        privacy.addView(actionRow(l("Borrar datos locales", "Erase local data"),
                l("Elimina reglas, actividad, configuración y claves",
                        "Deletes rules, activity, settings, and keys"),
                l("Borrar", "Erase"), view -> confirmEraseLocalData()));
        addCard(page, privacy);

        if (BuildConfig.NATIVE_BMUI1) {
            page.addView(sectionLabel(l("Aprendizaje local", "Local learning")));
            LinearLayout adaptive = card();
            String adaptiveStatus = safeCommand("IA_ESTADO");
            boolean adaptiveEnabled = "ON".equals(field(
                    adaptiveStatus, "ENABLED", "OFF"));
            adaptive.addView(switchSetting(l("Inteligencia adaptativa", "Adaptive intelligence"),
                    l("Aprende de tus decisiones sin cambiar las reglas",
                            "Learns from your choices without changing rules"),
                    adaptiveEnabled, (button, enabled) -> {
                        if (enabled) {
                            showAdaptiveConsentDialog(button);
                            return;
                        }
                        prefs.edit().putBoolean(PREF_AI_CONSENT, false).commit();
                        runSwitchMutation(button, false, "APRENDIZAJE_ADAPTATIVO OFF",
                                l("Aprendizaje desactivado", "Learning disabled"), null);
                    }));
            adaptive.addView(divider());
            adaptive.addView(labelValue(l("Modo", "Mode"),
                    l("Observación · las reglas siempre deciden",
                            "Observation · rules always decide"), colorGreenStrong));
            adaptive.addView(divider());
            adaptive.addView(labelValue(l("Muestras", "Samples"),
                    field(adaptiveStatus, "SAMPLES", "0"), colorMuted));
            adaptive.addView(divider());
            adaptive.addView(labelValue(l("Confianza", "Confidence"),
                    field(adaptiveStatus, "CONFIDENCE", "0") + "% · "
                            + field(adaptiveStatus, "SAMPLES", "0") + "/"
                            + field(adaptiveStatus, "MIN_SAMPLES", "30"), colorMuted));
            adaptive.addView(divider());
            adaptive.addView(labelValue(l("Revisiones", "Reviews"),
                    field(adaptiveStatus, "FEEDBACK", "0"), colorMuted));
            adaptive.addView(divider());
            adaptive.addView(actionRow(l("Borrar aprendizaje", "Erase learning"),
                    l("Conserva reglas y actividad", "Keeps rules and activity"),
                    l("Borrar", "Erase"), view -> {
                        prefs.edit().putBoolean(PREF_AI_CONSENT, false).commit();
                        runMutation("IA_REINICIAR",
                                l("Aprendizaje borrado", "Learning erased"));
                    }));
            addCard(page, adaptive);
        }

        page.addView(sectionLabel(l("Versión y parches", "Version and patches")));
        LinearLayout updates = card();
        String version = BuildConfig.NATIVE_BMUI1 ? safeCommand("VERSION") : "";
        updates.addView(labelValue("UnCall",
                field(version, "BLOCKMASTER_VERSION", BuildConfig.VERSION_NAME), colorText));
        updates.addView(divider());
        updates.addView(labelValue(l("Parche de seguridad", "Security patch"),
                field(version, "GENERAL_PATCH", "guard-2026.08.08-g5-privacy-ui"), colorGreenStrong));
        updates.addView(divider());
        updates.addView(labelValue(l("Parche criptográfico", "Cryptographic patch"),
                field(version, "CRYPTO_PATCH", "crypto-2026.08.03-a2-devicekeys"), colorGreenStrong));
        updates.addView(divider());
        updates.addView(labelValue(l("Canal", "Channel"),
                field(version, "UPDATE_CHANNEL", "stable"), colorMuted));
        TextView policy = text(l("Solo se aceptarán APK firmados con la identidad autorizada. "
                        + "Android pedirá confirmación antes de instalar una actualización.",
                "Only APKs signed by the authorized identity will be accepted. "
                        + "Android will request confirmation before installing an update."),
                13, colorMuted, Typeface.NORMAL);
        policy.setPadding(0, dp(16), 0, 0);
        updates.addView(policy);
        addCard(page, updates);
        return scroll(page);
    }

    private View brandHeader(String subtitle) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(16), dp(58), dp(28));
        ImageView mark = new ImageView(this);
        mark.setScaleType(ImageView.ScaleType.FIT_CENTER);
        mark.setContentDescription(l("Abrir asistente local", "Open local assistant"));
        int iconId = getResources().getIdentifier("uncall_icon", "drawable", getPackageName());
        if (iconId != 0) mark.setImageResource(iconId);
        mark.setBackground(rounded(dark ? Color.rgb(21, 38, 31) : Color.WHITE,
                18, colorBorder, 1));
        mark.setPadding(dp(5), dp(5), dp(5), dp(5));
        mark.setClickable(true);
        mark.setFocusable(true);
        mark.setForeground(settingsMenuRipple());
        exposeAsButton(mark);
        mark.setOnClickListener(view -> showLocalAssistantDialog());
        installPressFeedback(mark);
        row.addView(mark, new LinearLayout.LayoutParams(dp(56), dp(56)));
        LinearLayout words = new LinearLayout(this);
        words.setOrientation(LinearLayout.VERTICAL);
        TextView title = text("UnCall", 26, colorText, Typeface.BOLD);
        markHeading(title);
        words.addView(title);
        words.addView(text(subtitle, 13, colorMuted, Typeface.NORMAL));
        LinearLayout.LayoutParams wordsParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        wordsParams.leftMargin = dp(14);
        row.addView(words, wordsParams);
        return row;
    }

    private View pageHeader(String title, String subtitle) {
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.VERTICAL);
        header.setPadding(0, dp(16), dp(58), dp(28));
        TextView heading = text(title, 30, colorText, Typeface.BOLD);
        markHeading(heading);
        header.addView(heading);
        TextView sub = text(subtitle, 15, colorMuted, Typeface.NORMAL);
        sub.setPadding(0, dp(5), 0, 0);
        header.addView(sub);
        return header;
    }

    private View statCard(String label, String value, int accent, int soft) {
        LinearLayout box = card();
        box.setBackground(glass(22, false));
        TextView dot = text("●", 16, accent, Typeface.NORMAL);
        dot.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        box.addView(dot);
        TextView name = text(label, 15, colorText, Typeface.BOLD);
        name.setPadding(0, dp(6), 0, 0);
        box.addView(name);
        TextView number = text(value, 34, accent, Typeface.BOLD);
        number.setPadding(0, dp(6), 0, 0);
        box.addView(number);
        return box;
    }

    private LinearLayout decisionCard(String raw, long when) {
        LinearLayout box = card();
        DecisionView decision = formatDecision(raw);
        LinearLayout top = new LinearLayout(this);
        top.setOrientation(compactContent ? LinearLayout.VERTICAL : LinearLayout.HORIZONTAL);
        top.setGravity(compactContent ? Gravity.START : Gravity.CENTER_VERTICAL);
        TextView result = text(decision.blocked ? l("BLOQUEADA", "BLOCKED")
                        : decision.none ? l("SIN ACTIVIDAD", "NO ACTIVITY")
                        : l("PERMITIDA", "ALLOWED"),
                19, decision.none ? colorMuted : decision.blocked ? colorRed : colorGreenStrong,
                Typeface.BOLD);
        top.addView(result, compactContent
                ? new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT)
                : new LinearLayout.LayoutParams(0,
                        ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        if (!decision.none) {
            TextView state = pill(decision.blocked ? l("Protegida", "Protected")
                            : l("Permitida", "Allowed"),
                    decision.blocked ? colorRedSoft : colorGreenSoft,
                    decision.blocked ? colorRed : colorGreenStrong);
            LinearLayout.LayoutParams stateParams = wrapParams();
            if (compactContent) stateParams.topMargin = dp(8);
            top.addView(state, stateParams);
        }
        box.addView(top);
        TextView reason = text(decision.reason, 15, colorText, Typeface.NORMAL);
        reason.setPadding(0, dp(12), 0, dp(5));
        box.addView(reason);
        if (!decision.number.isEmpty()) {
            box.addView(text(decision.number, 14, colorMuted, Typeface.NORMAL));
        }
        if (when > 0L) {
            TextView time = text(android.text.format.DateFormat.format(
                    "dd/MM/yyyy · HH:mm:ss", when).toString(), 12, colorMuted, Typeface.NORMAL);
            time.setPadding(0, dp(10), 0, 0);
            box.addView(time);
        }
        return box;
    }

    private void addGroupedActivity(LinearLayout historyCard, List<DecisionEvent> events) {
        Map<String, ActivityDay> days = new LinkedHashMap<>();
        for (int i = events.size() - 1; i >= 0; i--) {
            DecisionEvent event = events.get(i);
            long wallTime = event.wallTime > 0L
                    ? event.wallTime : activityWallTime(event.seconds);
            String key = new java.text.SimpleDateFormat("yyyy-MM-dd", Locale.ROOT)
                    .format(new Date(wallTime));
            ActivityDay day = days.get(key);
            if (day == null) {
                day = new ActivityDay(key, wallTime);
                days.put(key, day);
            }
            (event.blocked ? day.blocked : day.allowed).add(event);
        }
        if (!activityExpansionInitialized && !days.isEmpty()) {
            expandedActivityDays.add(days.keySet().iterator().next());
            activityExpansionInitialized = true;
        }

        int dayIndex = 0;
        for (ActivityDay day : days.values()) {
            if (dayIndex++ > 0) historyCard.addView(divider());
            boolean expanded = expandedActivityDays.contains(day.key);
            LinearLayout dayHeader = expandableHeader(activityDayLabel(day.wallTime),
                    day.blocked.size() + day.allowed.size(), expanded,
                    view -> {
                        toggleSet(expandedActivityDays, day.key);
                        refreshCurrentPage();
                    }, colorGreenSoft, colorGreenStrong);
            historyCard.addView(dayHeader);
            if (!expanded) continue;
            if (!day.blocked.isEmpty()) {
                addActivityCategory(historyCard, day.key + "|blocked",
                        l("Bloqueadas", "Blocked"), day.blocked, true);
            }
            if (!day.allowed.isEmpty()) {
                addActivityCategory(historyCard, day.key + "|allowed",
                        l("Permitidas", "Allowed"), day.allowed, false);
            }
        }
    }

    private void addActivityCategory(LinearLayout parent, String key, String title,
            List<DecisionEvent> events, boolean blocked) {
        boolean expanded = expandedActivityGroups.contains(key);
        LinearLayout header = expandableHeader(title, events.size(), expanded,
                view -> {
                    toggleSet(expandedActivityGroups, key);
                    refreshCurrentPage();
                }, blocked ? colorRedSoft : colorGreenSoft,
                blocked ? colorRed : colorGreenStrong);
        LinearLayout.LayoutParams headerParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        headerParams.topMargin = dp(8);
        parent.addView(header, headerParams);
        if (!expanded) return;
        for (int i = 0; i < events.size(); i++) {
            parent.addView(activityRow(events.get(i)));
            if (i + 1 < events.size()) parent.addView(divider());
        }
    }

    private LinearLayout expandableHeader(String title, int count, boolean expanded,
            View.OnClickListener listener, int softColor, int strongColor) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(dp(13), dp(12), dp(12), dp(12));
        row.setBackground(pressable(withAlpha(softColor, dark ? 126 : 180),
                withAlpha(strongColor, dark ? 74 : 44), 16, Color.TRANSPARENT, 0));
        row.addView(text(title, 15, colorText, Typeface.BOLD),
                new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView countView = text(Integer.toString(count), 13, strongColor, Typeface.BOLD);
        countView.setPadding(dp(8), 0, dp(8), 0);
        countView.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        row.addView(countView);
        TextView arrow = text(expanded ? "⌃" : "⌄", 18, strongColor, Typeface.BOLD);
        arrow.setGravity(Gravity.CENTER);
        arrow.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        row.addView(arrow, new LinearLayout.LayoutParams(dp(28), dp(28)));
        row.setClickable(true);
        row.setFocusable(true);
        row.setOnClickListener(listener);
        exposeExpandable(row, title + ", " + count, expanded);
        installPressFeedback(row);
        return row;
    }

    private void toggleSet(Set<String> values, String key) {
        if (!values.remove(key)) values.add(key);
    }

    private long activityWallTime(long seconds) {
        long elapsed = SystemClock.elapsedRealtime() / 1000L;
        if (seconds <= 0L || seconds > elapsed) return System.currentTimeMillis();
        return System.currentTimeMillis() - ((elapsed - seconds) * 1000L);
    }

    private String activityDayLabel(long wallTime) {
        if (DateUtils.isToday(wallTime)) return l("Hoy", "Today");
        if (DateUtils.isToday(wallTime + DateUtils.DAY_IN_MILLIS)) return l("Ayer", "Yesterday");
        Locale locale = Locale.forLanguageTag(activeLanguageCode());
        return DateFormat.getDateInstance(DateFormat.MEDIUM, locale).format(new Date(wallTime));
    }

    private View activityRow(DecisionEvent event) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(13), 0, dp(13));
        TextView marker = text(event.blocked ? "×" : "✓", 18,
                event.blocked ? colorRed : colorGreenStrong, Typeface.BOLD);
        marker.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        marker.setGravity(Gravity.CENTER);
        marker.setBackground(rounded(event.blocked ? colorRedSoft : colorGreenSoft,
                16, Color.TRANSPARENT, 0));
        row.addView(marker, new LinearLayout.LayoutParams(dp(42), dp(42)));
        LinearLayout detail = new LinearLayout(this);
        detail.setOrientation(LinearLayout.VERTICAL);
        String name = activityDisplayName(event);
        String number = event.number.isEmpty() ? l("Número oculto", "Hidden number") : event.number;
        detail.addView(text(name.isEmpty() ? number : name, 15, colorText, Typeface.BOLD));
        String reason = reasonLabel(event.reason, event.rule);
        detail.addView(text(name.isEmpty() ? reason : number + " · " + reason,
                13, colorMuted, Typeface.NORMAL));
        LinearLayout.LayoutParams detailParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        detailParams.leftMargin = dp(12);
        row.addView(detail, detailParams);
        row.addView(text(relativeTime(event), 12, colorMuted, Typeface.NORMAL));
        row.setClickable(true);
        row.setFocusable(true);
        row.setContentDescription((name.isEmpty() ? number : name) + ". "
                + l("Abrir revisión de aprendizaje", "Open learning review"));
        row.setOnClickListener(view -> showAdaptiveFeedbackDialog(event));
        exposeAsButton(row);
        installPressFeedback(row);
        return row;
    }

    private String activityDisplayName(DecisionEvent event) {
        RuleProfileStore.Profile profile = RuleProfileStore.Profile.EMPTY;
        if ("NUMBER".equals(event.reason)) {
            profile = RuleProfileStore.load(this, "BLOCKLIST", event.rule);
        } else if ("PATTERN".equals(event.reason)) {
            profile = RuleProfileStore.load(this, "BLOCK_PATTERNS", event.rule);
        } else if ("WHITELIST".equals(event.reason)) {
            profile = RuleProfileStore.load(this, "WHITELIST", event.number);
        } else if ("ALLOW_PATTERN".equals(event.reason)) {
            profile = RuleProfileStore.load(this, "ALLOW_PATTERNS", event.rule);
        }
        if (!profile.name.isEmpty()) return profile.name;
        return ContactLookup.displayName(this, event.number);
    }

    private void addRuleSection(LinearLayout page, String groupId, String title, String subtitle,
            List<String> entries, String addCommand, String deleteCommand,
            String inputHint, boolean pattern) {
        LinearLayout box = card();
        LinearLayout heading = new LinearLayout(this);
        heading.setOrientation(LinearLayout.HORIZONTAL);
        heading.setGravity(Gravity.CENTER_VERTICAL);
        LinearLayout words = new LinearLayout(this);
        words.setOrientation(LinearLayout.VERTICAL);
        words.addView(text(title, 17, colorText, Typeface.BOLD));
        words.addView(text(subtitle + " · " + entries.size(), 13, colorMuted, Typeface.NORMAL));
        heading.addView(words, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        heading.addView(actionButton(l("Añadir", "Add"), false,
                view -> showAddRuleDialog(groupId, title, inputHint,
                        addCommand, deleteCommand, pattern)));
        box.addView(heading);
        if (entries.isEmpty()) {
            TextView empty = text(l("Lista vacía", "Empty list"),
                    14, colorMuted, Typeface.NORMAL);
            empty.setPadding(0, dp(18), 0, dp(5));
            box.addView(empty);
        } else {
            box.addView(space(10));
            for (String entry : entries) {
                box.addView(divider());
                LinearLayout row = new LinearLayout(this);
                row.setOrientation(LinearLayout.HORIZONTAL);
                row.setGravity(Gravity.CENTER_VERTICAL);
                row.setPadding(0, dp(11), 0, dp(7));
                RuleProfileStore.Profile profile = RuleProfileStore.load(this, groupId, entry);
                LinearLayout details = new LinearLayout(this);
                details.setOrientation(LinearLayout.VERTICAL);
                details.addView(text(profile.name.isEmpty() ? entry : profile.name,
                        15, colorText, profile.name.isEmpty() ? Typeface.NORMAL : Typeface.BOLD));
                if (!profile.name.isEmpty()) {
                    details.addView(text(entry, 13, colorMuted, Typeface.NORMAL));
                }
                if (!profile.address.isEmpty()) {
                    details.addView(text(profile.address, 13, colorMuted, Typeface.NORMAL));
                }
                row.addView(details, new LinearLayout.LayoutParams(0,
                        ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
                row.addView(actionButton(l("Cambiar", "Change"), false,
                        view -> showRuleProfileDialog(groupId, title, entry,
                                deleteCommand + entry)));
                row.setClickable(true);
                row.setFocusable(true);
                row.setOnClickListener(view -> showRuleProfileDialog(groupId, title, entry,
                        deleteCommand + entry));
                installPressFeedback(row);
                box.addView(row);
            }
        }
        addCard(page, box);
    }

    private void showAddRuleDialog(String groupId, String title, String hint,
            String command, String deleteCommand, boolean pattern) {
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        TextView heading = text(title, 22, colorText, Typeface.BOLD);
        sheet.addView(heading);
        TextView explanation = text(l("La regla se evaluará directamente en el núcleo nC.",
                "The rule will be evaluated directly by the nC core."),
                14, colorMuted, Typeface.NORMAL);
        explanation.setPadding(0, dp(7), 0, dp(18));
        sheet.addView(explanation);

        EditText input = profileInput(hint, true);
        input.setInputType(pattern ? InputType.TYPE_CLASS_TEXT
                : InputType.TYPE_CLASS_PHONE);
        sheet.addView(input, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        EditText name = profileInput(l("Nombre", "Name"), true);
        EditText address = profileInput(l("Dirección", "Address"), true);
        EditText notes = profileInput(l("Notas", "Notes"), false);
        addProfileInput(sheet, name);
        addProfileInput(sheet, address);
        addProfileInput(sheet, notes);

        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setPadding(dp(4), dp(8), dp(4), 0);
        error.setVisibility(View.GONE);
        sheet.addView(error);

        LinearLayout actions = dialogActions();
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false, view -> dialog.dismiss());
        TextView save = actionButton(l("Guardar", "Save"), true, null);
        save.setOnClickListener(view -> {
            String value = normalizeRuleInput(input.getText().toString(), pattern);
            if (value.isEmpty()) {
                showInlineError(error, l("Escribe una regla válida.", "Enter a valid rule."));
                return;
            }
            String failure = mutationError(command + value);
            if (failure != null) {
                showInlineError(error, friendlyMutationError(failure));
                return;
            }
            try {
                RuleProfileStore.save(this, groupId, value,
                        name.getText().toString(), address.getText().toString(),
                        notes.getText().toString());
            } catch (Exception persistenceFailure) {
                mutationError(deleteCommand + value);
                showInlineError(error, friendlyMutationError("ERR|PERSISTENCE"));
                return;
            }
            dialog.dismiss();
            toast(l("Regla guardada", "Rule saved"));
            refreshCurrentPage();
        });
        actions.addView(cancel, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams saveParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        saveParams.leftMargin = dp(10);
        actions.addView(save, saveParams);
        sheet.addView(actions);
        showBottomDialog(dialog, sheet);
        input.requestFocus();
        Window window = dialog.getWindow();
        if (window != null) window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_VISIBLE);
    }

    private void showRuleProfileDialog(String groupId, String title,
            String value, String deleteCommand) {
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(title, 22, colorText, Typeface.BOLD));
        TextView rule = text(value, 15, colorGreenStrong, Typeface.BOLD);
        rule.setPadding(0, dp(7), 0, dp(14));
        sheet.addView(rule);

        RuleProfileStore.Profile profile = RuleProfileStore.load(this, groupId, value);
        EditText name = profileInput(l("Nombre", "Name"), true);
        EditText address = profileInput(l("Dirección", "Address"), true);
        EditText notes = profileInput(l("Notas", "Notes"), false);
        name.setText(profile.name);
        address.setText(profile.address);
        notes.setText(profile.notes);
        sheet.addView(name);
        addProfileInput(sheet, address);
        addProfileInput(sheet, notes);

        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setPadding(dp(4), dp(8), dp(4), 0);
        error.setVisibility(View.GONE);
        sheet.addView(error);

        TextView remove = actionButton(l("Eliminar", "Delete"), false, view -> {
            dialog.dismiss();
            confirmDelete(groupId, title, value, deleteCommand);
        });
        remove.setTextColor(colorRed);
        LinearLayout.LayoutParams removeParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        removeParams.topMargin = dp(14);
        sheet.addView(remove, removeParams);

        LinearLayout actions = dialogActions();
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false, view -> dialog.dismiss());
        TextView save = actionButton(l("Guardar", "Save"), true, view -> {
            try {
                RuleProfileStore.save(this, groupId, value,
                        name.getText().toString(), address.getText().toString(),
                        notes.getText().toString());
                dialog.dismiss();
                toast(l("Regla guardada", "Rule saved"));
                refreshCurrentPage();
            } catch (Exception persistenceFailure) {
                showInlineError(error, friendlyMutationError("ERR|PERSISTENCE"));
            }
        });
        actions.addView(cancel, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams saveParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        saveParams.leftMargin = dp(10);
        actions.addView(save, saveParams);
        sheet.addView(actions);
        showBottomDialog(dialog, sheet);
    }

    private EditText profileInput(String hint, boolean singleLine) {
        EditText input = new EditText(this);
        input.setHint(hint);
        input.setSingleLine(singleLine);
        input.setMaxLines(singleLine ? 1 : 3);
        input.setTextColor(colorText);
        input.setHintTextColor(colorMuted);
        input.setTextSize(15);
        input.setPadding(dp(16), dp(13), dp(16), dp(13));
        input.setBackground(rounded(dark ? Color.rgb(18, 24, 22) : colorBackground,
                16, colorBorder, 1));
        input.setInputType(InputType.TYPE_CLASS_TEXT
                | (singleLine ? 0 : InputType.TYPE_TEXT_FLAG_MULTI_LINE));
        applyTextDirection(input);
        return input;
    }

    private void addProfileInput(LinearLayout sheet, View input) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.topMargin = dp(10);
        sheet.addView(input, params);
    }

    private void confirmDelete(String groupId, String group, String value, String command) {
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(l("Eliminar regla", "Delete rule"), 22, colorText, Typeface.BOLD));
        TextView detail = text(group + "\n" + value, 15, colorMuted, Typeface.NORMAL);
        detail.setPadding(0, dp(10), 0, dp(18));
        sheet.addView(detail);
        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setVisibility(View.GONE);
        sheet.addView(error);
        LinearLayout actions = dialogActions();
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false, view -> dialog.dismiss());
        TextView remove = actionButton(l("Eliminar", "Delete"), true, view -> {
            String failure = mutationError(command);
            if (failure != null) {
                showInlineError(error, friendlyMutationError(failure));
                return;
            }
            RuleProfileStore.delete(this, groupId, value);
            dialog.dismiss();
            toast(l("Regla eliminada", "Rule deleted"));
            refreshCurrentPage();
        });
        remove.setBackground(rounded(colorRed, 16, colorRed, 0));
        actions.addView(cancel, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams removeParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        removeParams.leftMargin = dp(10);
        actions.addView(remove, removeParams);
        sheet.addView(actions);
        showBottomDialog(dialog, sheet);
    }

    private void confirmEraseLocalData() {
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(l("Borrar todos los datos locales", "Erase all local data"),
                22, colorText, Typeface.BOLD));
        TextView detail = text(l("Esta acción elimina reglas, actividad, preferencias y "
                        + "claves del dispositivo. No se puede deshacer.",
                "This deletes rules, activity, preferences, and device keys. "
                        + "It cannot be undone."), 15, colorMuted, Typeface.NORMAL);
        detail.setPadding(0, dp(10), 0, dp(18));
        sheet.addView(detail);
        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setVisibility(View.GONE);
        sheet.addView(error);
        LinearLayout actions = dialogActions();
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false, view -> dialog.dismiss());
        TextView erase = actionButton(l("Borrar", "Erase"), true, view -> {
            try {
                NativeBridge.ensureInitialized(this);
                NativeBridge.command("LIMPIAR_HISTORIAL");
                NativeBridge.command("LIMPIAR");
                if (BuildConfig.REMOTE_CONTROL_ENABLED) {
                    stopService(new Intent(this, BlockMasterService.class));
                }
                BlocklistStore.eraseAll(this);
                RuleProfileStore.eraseAll(this);
                ActivityStore.eraseAll(this);
                if (!prefs.edit().clear().commit()) {
                    throw new IOException("preferences erase failed");
                }
                SecurityVault.destroyDeviceSecrets();
                dialog.dismiss();
                toast(l("Datos locales borrados", "Local data erased"));
                recreate();
            } catch (Exception failure) {
                showInlineError(error, l("No se pudieron borrar todos los datos.",
                        "Not all local data could be erased."));
            }
        });
        erase.setBackground(rounded(colorRed, 16, colorRed, 0));
        actions.addView(cancel, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams eraseParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        eraseParams.leftMargin = dp(10);
        actions.addView(erase, eraseParams);
        sheet.addView(actions);
        showBottomDialog(dialog, sheet);
    }

    private void showAdaptiveConsentDialog(android.widget.CompoundButton button) {
        button.setTag(Boolean.TRUE);
        button.setChecked(false);
        button.setTag(null);

        Dialog dialog = new Dialog(this);
        final boolean[] consentAccepted = {false};
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(l("Activar aprendizaje local", "Enable local learning"),
                22, colorText, Typeface.BOLD));
        TextView disclosure = text(l(
                "UnCall usará el número entrante, si pertenece a un contacto, la hora y "
                        + "tus revisiones explícitas para personalizar sugerencias. Los datos "
                        + "permanecen cifrados en este dispositivo, nunca se envían y nunca "
                        + "bloquean automáticamente.",
                "UnCall will use the incoming number, whether it is a contact, the time, and "
                        + "your explicit reviews to personalize suggestions. Data stays "
                        + "encrypted on this device, is never sent, and never blocks "
                        + "automatically."), 15, colorMuted, Typeface.NORMAL);
        disclosure.setPadding(0, dp(10), 0, dp(8));
        sheet.addView(disclosure);
        TextView guarantee = text(l(
                "No se lee el historial general de llamadas. Puedes desactivarlo o borrar "
                        + "el aprendizaje en cualquier momento.",
                "The general call log is not read. You can disable or erase learning at any "
                        + "time."), 14, colorGreenStrong, Typeface.BOLD);
        guarantee.setPadding(0, dp(6), 0, dp(8));
        sheet.addView(guarantee);
        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setVisibility(View.GONE);
        sheet.addView(error);

        LinearLayout actions = dialogActions();
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false,
                view -> dialog.dismiss());
        TextView enable = actionButton(l("Activar", "Enable"), true, view -> {
            if (!prefs.edit().putBoolean(PREF_AI_CONSENT, true).commit()) {
                showInlineError(error, l("No se pudo guardar el consentimiento.",
                        "Consent could not be saved."));
                return;
            }
            consentAccepted[0] = true;
            button.setTag(Boolean.TRUE);
            button.setChecked(true);
            button.setTag(null);
            dialog.dismiss();
            runSwitchMutation(button, true, "APRENDIZAJE_ADAPTATIVO ON",
                    l("Aprendizaje activado", "Learning enabled"), null);
        });
        actions.addView(cancel, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams enableParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        enableParams.leftMargin = dp(10);
        actions.addView(enable, enableParams);
        sheet.addView(actions);
        dialog.setOnDismissListener(ignored -> {
            if (!consentAccepted[0]) enforceAdaptiveConsentOff(button);
        });
        showBottomDialog(dialog, sheet);
    }

    private void enforceAdaptiveConsentOff(android.widget.CompoundButton button) {
        prefs.edit().putBoolean(PREF_AI_CONSENT, false).commit();
        button.setTag(Boolean.TRUE);
        button.setChecked(false);
        button.setTag(null);
        button.setClickable(false);
        new Thread(() -> {
            final String failure;
            synchronized (mutationLock) {
                failure = mutationError("APRENDIZAJE_ADAPTATIVO OFF");
            }
            runOnUiThread(() -> {
                button.setClickable(true);
                if (failure != null) toast(friendlyMutationError(failure));
            });
        }, "UnCall-ai-consent-cancel").start();
    }

    private void showAdaptiveFeedbackDialog(DecisionEvent event) {
        String status = safeCommand("IA_ESTADO");
        if (!"ON".equals(field(status, "ENABLED", "OFF"))) {
            toast(l("Activa primero el aprendizaje local en Ajustes.",
                    "Enable local learning in Settings first."));
            return;
        }
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(l("Ayudar al aprendizaje local", "Help local learning"),
                22, colorText, Typeface.BOLD));
        TextView explanation = text(l(
                "¿Fue correcta esta decisión? Tu revisión permanece cifrada en este dispositivo.",
                "Was this decision right? Your review stays encrypted on this device."),
                15, colorMuted, Typeface.NORMAL);
        explanation.setPadding(0, dp(8), 0, dp(12));
        sheet.addView(explanation);
        String display = activityDisplayName(event);
        if (display.isEmpty()) {
            display = event.number.isEmpty() ? l("Número oculto", "Hidden number") : event.number;
        }
        sheet.addView(labelValue(display,
                event.blocked ? l("Bloqueada", "Blocked") : l("Permitida", "Allowed"),
                event.blocked ? colorRed : colorGreenStrong));
        TextView error = text("", 13, colorRed, Typeface.BOLD);
        error.setVisibility(View.GONE);
        sheet.addView(error);
        LinearLayout actions = dialogActions();
        TextView change = actionButton(l("Cambiar", "Change"), false,
                view -> recordAdaptiveFeedback(event, !event.blocked, dialog, error));
        TextView correct = actionButton(l("Correcta", "Correct"), true,
                view -> recordAdaptiveFeedback(event, event.blocked, dialog, error));
        actions.addView(change, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        LinearLayout.LayoutParams correctParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        correctParams.leftMargin = dp(10);
        actions.addView(correct, correctParams);
        sheet.addView(actions);
        showBottomDialog(dialog, sheet);
    }

    private void recordAdaptiveFeedback(DecisionEvent event, boolean expectedBlock,
            Dialog dialog, TextView error) {
        String number = event.number == null || event.number.isEmpty()
                ? "-" : event.number.replace("|", "").replace(" ", "");
        boolean contact = "CONTACT".equals(event.reason);
        boolean repeated = "URGENCY_REPEAT".equals(event.reason);
        long wallTime = event.wallTime > 0L ? event.wallTime : System.currentTimeMillis();
        long seconds = Math.max(1L, Math.min(Integer.MAX_VALUE, wallTime / 1000L));
        String command = "IA_FEEDBACK " + (expectedBlock ? "BLOCK" : "ALLOW")
                + "|" + (contact ? "1" : "0") + "|" + (repeated ? "1" : "0")
                + "|" + seconds + "|" + number;
        String failure = mutationError(command);
        if (failure != null) {
            showInlineError(error, friendlyMutationError(failure));
            return;
        }
        dialog.dismiss();
        toast(l("Revisión guardada", "Review saved"));
        refreshCurrentPage();
    }

    private void showLanguageDialog() {
        Dialog dialog = new Dialog(this);
        LinearLayout sheet = dialogSheet();
        sheet.addView(text(l("Idioma", "Language"), 22, colorText, Typeface.BOLD));
        TextView explanation = text(l("Selecciona el idioma de toda la interfaz. Hay 14 idiomas disponibles.",
                "Choose the language for the whole interface. Fourteen languages are available."),
                14, colorMuted, Typeface.NORMAL);
        explanation.setPadding(0, dp(7), 0, dp(12));
        sheet.addView(explanation);

        LinearLayout languages = new LinearLayout(this);
        languages.setOrientation(LinearLayout.VERTICAL);
        String selected = prefs.getString(PREF_LANGUAGE_MODE, LANGUAGE_SYSTEM);
        for (String code : Translations.CODES) {
            String label = ("system".equals(code) ? l("Sistema", "System") + " · " : "")
                    + Translations.languageName(code);
            TextView option = text((code.equals(selected) ? "✓  " : "    ") + label,
                    15, code.equals(selected) ? colorGreenStrong : colorText, Typeface.BOLD);
            option.setGravity(Gravity.CENTER_VERTICAL);
            option.setPadding(dp(14), dp(13), dp(14), dp(13));
            option.setBackground(navBackground(code.equals(selected)));
            option.setClickable(true);
            option.setFocusable(true);
            option.setOnClickListener(view -> {
                prefs.edit().putString(PREF_LANGUAGE_MODE, code).commit();
                dialog.dismiss();
                applyLanguageDirection();
                rebuildShellKeepingPage();
            });
            installPressFeedback(option);
            languages.addView(option, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        }
        ScrollView list = new ScrollView(this);
        list.setFillViewport(false);
        list.setVerticalScrollBarEnabled(false);
        list.addView(languages);
        sheet.addView(list, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(410)));
        TextView close = actionButton(l("Cerrar", "Close"), false, view -> dialog.dismiss());
        LinearLayout.LayoutParams closeParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        closeParams.topMargin = dp(14);
        sheet.addView(close, closeParams);
        showBottomDialog(dialog, sheet);
    }

    private void showLocalAssistantDialog() {
        Dialog dialog = new Dialog(this);
        LinearLayout screen = new LinearLayout(this);
        screen.setOrientation(LinearLayout.VERTICAL);
        screen.setPadding(dp(18), dp(9), dp(18), dp(14));
        screen.setBackground(rounded(dark ? Color.argb(234, 29, 40, 35)
                        : Color.argb(246, 255, 255, 255),
                30, dark ? Color.argb(46, 255, 255, 255)
                        : Color.argb(46, 15, 156, 73), 1));

        View handle = new View(this);
        handle.setBackground(rounded(withAlpha(colorMuted, dark ? 90 : 55), 3,
                Color.TRANSPARENT, 0));
        LinearLayout.LayoutParams handleParams = new LinearLayout.LayoutParams(dp(38), dp(4));
        handleParams.gravity = Gravity.CENTER_HORIZONTAL;
        handleParams.bottomMargin = dp(11);
        screen.addView(handle, handleParams);

        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(Gravity.CENTER_VERTICAL);
        LinearLayout iconSurface = new LinearLayout(this);
        iconSurface.setGravity(Gravity.CENTER);
        iconSurface.setBackground(rounded(withAlpha(colorGreenSoft, dark ? 190 : 218),
                16, withAlpha(colorGreen, 38), 1));
        ImageView icon = new ImageView(this);
        icon.setScaleType(ImageView.ScaleType.FIT_CENTER);
        int iconId = getResources().getIdentifier("uncall_icon", "drawable", getPackageName());
        if (iconId != 0) icon.setImageResource(iconId);
        icon.setPadding(dp(5), dp(5), dp(5), dp(5));
        icon.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        iconSurface.addView(icon, new LinearLayout.LayoutParams(dp(36), dp(36)));
        header.addView(iconSurface, new LinearLayout.LayoutParams(dp(48), dp(48)));

        LinearLayout heading = new LinearLayout(this);
        heading.setOrientation(LinearLayout.VERTICAL);
        TextView title = text("Nuinui", 21, colorText, Typeface.BOLD);
        markHeading(title);
        heading.addView(title);
        heading.addView(text(l("Asistente de UnCall · local por defecto",
                "UnCall assistant · local by default"), 12, colorMuted, Typeface.NORMAL));
        LinearLayout.LayoutParams headingParams = new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        headingParams.leftMargin = dp(12);
        header.addView(heading, headingParams);

        TextView close = text("×", 19, colorText, Typeface.NORMAL);
        close.setGravity(Gravity.CENTER);
        close.setBackground(pressable(dark ? Color.argb(152, 38, 50, 44)
                        : Color.argb(174, 255, 255, 255),
                withAlpha(colorGreen, 34), 16, withAlpha(colorBorder, 145), 1));
        close.setClickable(true);
        close.setFocusable(true);
        close.setOnClickListener(view -> screen.animate()
                .alpha(0f).translationY(dp(18))
                .setInterpolator(new DecelerateInterpolator(1.5f))
                .setDuration(reducedMotion ? 1L : 180L)
                .withEndAction(dialog::dismiss).start());
        installPressFeedback(close);
        close.setContentDescription(l("Cerrar", "Close"));
        header.addView(close, new LinearLayout.LayoutParams(dp(44), dp(44)));
        screen.addView(header);

        ScrollView transcriptScroll = new ScrollView(this);
        transcriptScroll.setFillViewport(true);
        transcriptScroll.setClipToPadding(false);
        transcriptScroll.setVerticalScrollBarEnabled(false);
        LinearLayout transcript = new LinearLayout(this);
        transcript.setOrientation(LinearLayout.VERTICAL);
        transcript.setPadding(0, dp(13), 0, dp(14));
        transcriptScroll.addView(transcript, new ScrollView.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        screen.addView(transcriptScroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        appendChatBubble(transcript, assistantText(safeCommand("ASISTENTE @welcome")), false);

        EditText input = new EditText(this);
        input.setTextColor(colorText);
        input.setHintTextColor(colorMuted);
        input.setHint(l("Escribe a Nuinui", "Message Nuinui"));
        input.setSingleLine(false);
        input.setMaxLines(3);
        input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
                | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        input.setFilters(new InputFilter[] {new InputFilter.LengthFilter(300)});
        input.setPadding(dp(16), dp(12), dp(16), dp(12));
        input.setBackground(rounded(dark ? Color.argb(184, 27, 37, 33)
                        : Color.argb(194, 255, 255, 255),
                22, withAlpha(colorBorder, 160), 1));
        applyTextDirection(input);

        HorizontalScrollView suggestionScroll = new HorizontalScrollView(this);
        suggestionScroll.setHorizontalScrollBarEnabled(false);
        suggestionScroll.setClipToPadding(false);
        LinearLayout suggestions = new LinearLayout(this);
        suggestions.setOrientation(LinearLayout.HORIZONTAL);
        suggestions.setPadding(0, dp(8), dp(12), dp(8));
        addNuinuiChip(suggestions, l("Última llamada", "Latest call"), view ->
                sendNuinuiPrompt(dialog, "@calls|last", l("Última llamada", "Latest call"),
                        transcript, transcriptScroll, input));
        addNuinuiChip(suggestions, l("Gestionar reglas", "Manage rules"), view ->
                sendNuinuiPrompt(dialog, "@rules", l("Gestionar reglas", "Manage rules"),
                        transcript, transcriptScroll, input));
        addNuinuiChip(suggestions, l("Estado", "Status"), view ->
                sendNuinuiPrompt(dialog, "@status", l("Estado", "Status"),
                        transcript, transcriptScroll, input));
        addNuinuiChip(suggestions, l("Antispam", "Antispam"), view ->
                sendNuinuiPrompt(dialog, "@spam", l("Antispam", "Antispam"),
                        transcript, transcriptScroll, input));
        addNuinuiChip(suggestions, l("Privacidad", "Privacy"), view ->
                sendNuinuiPrompt(dialog, "@privacy", l("Privacidad", "Privacy"),
                        transcript, transcriptScroll, input));
        addNuinuiChip(suggestions, l("Buscar en web", "Search the web"), view -> {
            input.setTag(null);
            input.setText(l("Buscar en web: ", "Search the web: "));
            input.setSelection(input.length());
            input.requestFocus();
        });
        suggestionScroll.addView(suggestions, new HorizontalScrollView.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        screen.addView(suggestionScroll);

        LinearLayout composer = new LinearLayout(this);
        composer.setOrientation(LinearLayout.HORIZONTAL);
        composer.setGravity(Gravity.BOTTOM);
        composer.setPadding(0, dp(4), 0, 0);
        composer.addView(input, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView send = text("↑", 22, Color.WHITE, Typeface.BOLD);
        send.setGravity(Gravity.CENTER);
        send.setBackground(pressable(colorGreen, withAlpha(Color.WHITE, 40), 25,
                Color.TRANSPARENT, 0));
        send.setClickable(true);
        send.setFocusable(true);
        send.setContentDescription(l("Enviar", "Send"));
        installPressFeedback(send);
        LinearLayout.LayoutParams sendParams = new LinearLayout.LayoutParams(dp(52), dp(52));
        sendParams.leftMargin = dp(8);
        composer.addView(send, sendParams);
        View.OnClickListener sendAction = view -> {
            String entered = input.getText().toString().trim();
            if (entered.isEmpty()) return;
            String prompt = entered.replace('\n', ' ').replace('\r', ' ');
            String visiblePrompt = entered;
            Object draft = input.getTag();
            if (draft instanceof String[]) {
                String[] rule = (String[]) draft;
                String normalized = normalizeRuleInput(entered, "PATTERN".equals(rule[2]));
                if (normalized.isEmpty()) {
                    toast(l("Introduce un número o patrón válido",
                            "Enter a valid number or pattern"));
                    return;
                }
                prompt = "@rule|" + rule[0] + "|" + rule[1] + "|" + rule[2]
                        + "|" + normalized;
                visiblePrompt = rule[3] + ": " + normalized;
            }
            input.setText("");
            input.setTag(null);
            input.setHint(l("Escribe a Nuinui", "Message Nuinui"));
            sendNuinuiPrompt(dialog, prompt, visiblePrompt,
                    transcript, transcriptScroll, input);
        };
        send.setOnClickListener(sendAction);
        input.setOnEditorActionListener((view, actionId, event) -> {
            sendAction.onClick(send);
            return true;
        });
        screen.addView(composer);

        dialog.setContentView(screen);
        dialog.setCanceledOnTouchOutside(true);
        dialog.show();
        Window window = dialog.getWindow();
        if (window != null) {
            window.setBackgroundDrawableResource(android.R.color.transparent);
            window.setDimAmount(dark ? 0.38f : 0.24f);
            window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
            if (Build.VERSION.SDK_INT >= 31) {
                WindowManager.LayoutParams attributes = window.getAttributes();
                attributes.setBlurBehindRadius(dp(22));
                window.setAttributes(attributes);
                window.addFlags(WindowManager.LayoutParams.FLAG_BLUR_BEHIND);
            }
            window.setGravity(Gravity.BOTTOM);
            window.getDecorView().setPadding(dp(14), 0, dp(14), dp(14));
            int dialogWidth = Math.min(getResources().getDisplayMetrics().widthPixels,
                    dp(tabletContent ? 620 : 720));
            int dialogHeight = Math.round(
                    getResources().getDisplayMetrics().heightPixels * 0.82f);
            window.setLayout(dialogWidth, dialogHeight);
            window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE);
        }
        screen.setAlpha(0f);
        screen.setTranslationY(dp(24));
        screen.animate().alpha(1f).translationY(0f)
                .setInterpolator(new DecelerateInterpolator(1.7f))
                .setDuration(reducedMotion ? 1L : 280L).start();
    }

    private void addNuinuiChip(LinearLayout row, String label, View.OnClickListener action) {
        TextView chip = text(label, 12, colorText, Typeface.BOLD);
        chip.setGravity(Gravity.CENTER);
        chip.setPadding(dp(14), dp(10), dp(14), dp(10));
        chip.setBackground(pressable(dark ? Color.rgb(29, 40, 35) : Color.WHITE,
                withAlpha(colorGreen, 25), 18, colorBorder, 1));
        chip.setClickable(true);
        chip.setFocusable(true);
        chip.setOnClickListener(action);
        exposeAsButton(chip);
        installPressFeedback(chip);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, dp(42));
        params.rightMargin = dp(7);
        row.addView(chip, params);
    }

    private void sendNuinuiPrompt(Dialog dialog, String prompt, String visiblePrompt,
            LinearLayout transcript, ScrollView transcriptScroll, EditText input) {
        appendChatBubble(transcript, visiblePrompt, true);
        String response = safeCommand("ASISTENTE " + prompt);
        String intent = field(response, "INTENT", "UNKNOWN");
        if ("CALL_QUERY".equals(intent)) {
            appendChatBubble(transcript, nuinuiCallAnswer(visiblePrompt, response), false);
        } else {
            appendChatBubble(transcript, assistantText(response), false);
        }
        if ("PROPOSE_RULE".equals(intent)) {
            appendNuinuiRuleConfirmation(response, transcript, transcriptScroll);
        } else if ("RULES".equals(intent)) {
            appendNuinuiRuleActions(transcript, transcriptScroll, input);
        } else if ("WEB_TELEPHONY".equals(intent) || "WEB_GENERAL".equals(intent)) {
            appendNuinuiWebConsent(dialog, cleanNuinuiWebQuery(visiblePrompt), transcript,
                    transcriptScroll);
        }
        scrollTranscript(transcriptScroll);
    }

    private void appendChatBubble(LinearLayout transcript, String message, boolean user) {
        LinearLayout line = new LinearLayout(this);
        line.setOrientation(LinearLayout.HORIZONTAL);
        line.setGravity(user ? Gravity.END : Gravity.TOP);
        if (!user) {
            TextView avatar = text("N", 12, colorGreenStrong, Typeface.BOLD);
            avatar.setGravity(Gravity.CENTER);
            avatar.setBackground(rounded(colorGreenSoft, 14, Color.TRANSPARENT, 0));
            avatar.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
            line.addView(avatar, new LinearLayout.LayoutParams(dp(28), dp(28)));
        }
        TextView bubble = text(message, 14, user ? Color.WHITE : colorText, Typeface.NORMAL);
        bubble.setPadding(dp(14), dp(11), dp(14), dp(11));
        bubble.setMaxWidth((int) (getResources().getDisplayMetrics().widthPixels * 0.76f));
        bubble.setBackground(rounded(user ? colorGreen : (dark
                ? Color.rgb(31, 43, 37) : Color.rgb(239, 246, 242)), 18,
                Color.TRANSPARENT, 0));
        LinearLayout.LayoutParams bubbleParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        if (!user) bubbleParams.leftMargin = dp(8);
        line.addView(bubble, bubbleParams);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.topMargin = dp(7);
        transcript.addView(line, params);
    }

    private void appendNuinuiRuleActions(LinearLayout transcript,
            ScrollView transcriptScroll, EditText input) {
        LinearLayout card = nuinuiInlineCard();
        card.addView(text(l("Elige una acción", "Choose an action"),
                14, colorText, Typeface.BOLD));
        String[] labels = {
                l("Añadir bloqueado", "Add blocked"), l("Quitar bloqueado", "Remove blocked"),
                l("Añadir patrón bloqueado", "Add blocked pattern"),
                l("Quitar patrón bloqueado", "Remove blocked pattern"),
                l("Añadir permitido", "Add allowed"), l("Quitar permitido", "Remove allowed"),
                l("Añadir patrón permitido", "Add allowed pattern"),
                l("Quitar patrón permitido", "Remove allowed pattern")
        };
        String[][] actions = {
                {"ADD", "BLOCK", "NUMBER"}, {"REMOVE", "BLOCK", "NUMBER"},
                {"ADD", "BLOCK", "PATTERN"}, {"REMOVE", "BLOCK", "PATTERN"},
                {"ADD", "ALLOW", "NUMBER"}, {"REMOVE", "ALLOW", "NUMBER"},
                {"ADD", "ALLOW", "PATTERN"}, {"REMOVE", "ALLOW", "PATTERN"}
        };
        for (int rowIndex = 0; rowIndex < 4; rowIndex++) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setPadding(0, rowIndex == 0 ? dp(10) : dp(6), 0, 0);
            for (int column = 0; column < 2; column++) {
                int index = rowIndex * 2 + column;
                TextView action = text(labels[index], 12, colorText, Typeface.BOLD);
                action.setGravity(Gravity.CENTER);
                action.setMinHeight(dp(46));
                action.setPadding(dp(8), dp(8), dp(8), dp(8));
                action.setBackground(pressable(dark ? Color.rgb(38, 50, 44) : Color.WHITE,
                        withAlpha(colorGreen, 24), 15, colorBorder, 1));
                action.setClickable(true);
                action.setFocusable(true);
                final int selected = index;
                action.setOnClickListener(view -> prepareNuinuiRuleDraft(input,
                        actions[selected], labels[selected]));
                exposeAsButton(action);
                installPressFeedback(action);
                LinearLayout.LayoutParams actionParams = new LinearLayout.LayoutParams(
                        0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
                if (column > 0) actionParams.leftMargin = dp(6);
                row.addView(action, actionParams);
            }
            card.addView(row);
        }
        addNuinuiInlineCard(transcript, card);
        scrollTranscript(transcriptScroll);
    }

    private void prepareNuinuiRuleDraft(EditText input, String[] action, String label) {
        input.setText("");
        input.setTag(new String[] {action[0], action[1], action[2], label});
        input.setHint("PATTERN".equals(action[2])
                ? l("Escribe el patrón", "Enter the pattern")
                : l("Escribe el número", "Enter the number"));
        input.requestFocus();
        input.announceForAccessibility(label);
    }

    private void appendNuinuiRuleConfirmation(String response, LinearLayout transcript,
            ScrollView transcriptScroll) {
        LinearLayout card = nuinuiInlineCard();
        card.addView(text(l("Revisar cambio", "Review change"),
                14, colorText, Typeface.BOLD));
        TextView detail = text(nuinuiRuleDescription(response),
                13, colorMuted, Typeface.NORMAL);
        detail.setPadding(0, dp(5), 0, dp(12));
        card.addView(detail);
        LinearLayout actions = new LinearLayout(this);
        actions.setOrientation(LinearLayout.HORIZONTAL);
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false, view -> {
            card.setVisibility(View.GONE);
            appendChatBubble(transcript, l("Cambio cancelado", "Change cancelled"), false);
            scrollTranscript(transcriptScroll);
        });
        actions.addView(cancel, new LinearLayout.LayoutParams(0, dp(48), 1f));
        TextView confirm = actionButton(l("Confirmar cambio", "Confirm change"), true, null);
        LinearLayout.LayoutParams confirmParams = new LinearLayout.LayoutParams(0, dp(48), 1f);
        confirmParams.leftMargin = dp(8);
        actions.addView(confirm, confirmParams);
        confirm.setOnClickListener(view -> {
            String command = nuinuiRuleCommand(response);
            if (command.isEmpty()) return;
            confirm.setEnabled(false);
            confirm.setAlpha(0.55f);
            new Thread(() -> {
                final String failure;
                synchronized (mutationLock) {
                    failure = mutationError(command);
                }
                runOnUiThread(() -> {
                    appendChatBubble(transcript, failure == null
                            ? l("Cambio guardado", "Change saved")
                            : friendlyMutationError(failure), false);
                    card.setVisibility(View.GONE);
                    refreshCurrentPage();
                    scrollTranscript(transcriptScroll);
                });
            }, "nuinui-confirmed-rule").start();
        });
        card.addView(actions);
        addNuinuiInlineCard(transcript, card);
    }

    private String nuinuiRuleDescription(String response) {
        String operation = "REMOVE".equals(field(response, "OP", "ADD"))
                ? l("Quitar", "Remove") : l("Añadir", "Add");
        String list = "ALLOW".equals(field(response, "LIST", "BLOCK"))
                ? l("permitidos", "allowed") : l("bloqueados", "blocked");
        String kind = "PATTERN".equals(field(response, "KIND", "NUMBER"))
                ? l("patrón", "pattern") : l("número", "number");
        return operation + " · " + kind + " · " + list + "\n"
                + field(response, "VALUE", "");
    }

    private String nuinuiRuleCommand(String response) {
        String operation = field(response, "OP", "");
        String list = field(response, "LIST", "");
        String kind = field(response, "KIND", "");
        String value = field(response, "VALUE", "");
        if (value.isEmpty()) return "";
        if ("ADD".equals(operation) && "BLOCK".equals(list) && "NUMBER".equals(kind)) {
            return "AGREGAR " + value;
        }
        if ("REMOVE".equals(operation) && "BLOCK".equals(list) && "NUMBER".equals(kind)) {
            return "ELIMINAR " + value;
        }
        if ("ADD".equals(operation) && "BLOCK".equals(list) && "PATTERN".equals(kind)) {
            return "BLOQUEAR_POR_PATRON " + value;
        }
        if ("REMOVE".equals(operation) && "BLOCK".equals(list) && "PATTERN".equals(kind)) {
            return "DESBLOQUEAR_PATRON " + value;
        }
        if ("ADD".equals(operation) && "ALLOW".equals(list) && "NUMBER".equals(kind)) {
            return "AGREGAR_WHITELIST " + value;
        }
        if ("REMOVE".equals(operation) && "ALLOW".equals(list) && "NUMBER".equals(kind)) {
            return "ELIMINAR_WHITELIST " + value;
        }
        if ("ADD".equals(operation) && "ALLOW".equals(list) && "PATTERN".equals(kind)) {
            return "PERMITIR_POR_PATRON " + value;
        }
        if ("REMOVE".equals(operation) && "ALLOW".equals(list) && "PATTERN".equals(kind)) {
            return "QUITAR_PERMISO_PATRON " + value;
        }
        return "";
    }

    private String nuinuiCallAnswer(String userQuery, String response) {
        List<DecisionEvent> events = loadActivityEvents();
        if (events.isEmpty()) {
            return l("Todavía no hay llamadas evaluadas por UnCall.",
                    "UnCall has not evaluated any calls yet.");
        }
        String requestedNumber = field(response, "NUMBER", "");
        String query = userQuery == null ? "" : userQuery.toLowerCase(Locale.ROOT);
        DecisionEvent selected = null;
        for (int index = events.size() - 1; index >= 0; index--) {
            DecisionEvent candidate = events.get(index);
            if (!requestedNumber.isEmpty()
                    && !digits(candidate.number).equals(digits(requestedNumber))) continue;
            if (requestedNumber.isEmpty()) {
                String name = activityDisplayName(candidate).toLowerCase(Locale.ROOT);
                boolean named = name.length() >= 3 && query.contains(name);
                boolean blocked = query.contains("bloquead") || query.contains("blocked");
                boolean allowed = query.contains("permitid") || query.contains("allowed");
                if (!name.isEmpty() && named) {
                    selected = candidate;
                    break;
                }
                if (blocked && !candidate.blocked) continue;
                if (allowed && candidate.blocked) continue;
            }
            selected = candidate;
            break;
        }
        if (selected == null) {
            return l("No encuentro una llamada evaluada que coincida con esa pregunta.",
                    "I cannot find an evaluated call matching that question.");
        }
        long when = selected.wallTime > 0L
                ? selected.wallTime : activityWallTime(selected.seconds);
        Locale locale = Locale.forLanguageTag(activeLanguageCode());
        String date = DateFormat.getDateTimeInstance(DateFormat.MEDIUM,
                DateFormat.SHORT, locale).format(new Date(when));
        String name = activityDisplayName(selected);
        String number = selected.number.isEmpty()
                ? l("Número oculto", "Hidden number") : selected.number;
        return l("Llamada evaluada por UnCall", "Call evaluated by UnCall") + "\n\n"
                + l("De", "From") + ": " + (name.isEmpty() ? number : name) + "\n"
                + (name.isEmpty() ? "" : l("Número", "Number") + ": " + number + "\n")
                + l("Hora", "Time") + ": " + date + "\n"
                + l("Resultado", "Result") + ": "
                + (selected.blocked ? l("Bloqueada", "Blocked") : l("Permitida", "Allowed"))
                + "\n" + l("Motivo", "Reason") + ": "
                + reasonLabel(selected.reason, selected.rule) + "\n\n"
                + l("Fuente: actividad cifrada de UnCall",
                        "Source: encrypted UnCall activity");
    }

    private String digits(String value) {
        if (value == null) return "";
        StringBuilder normalized = new StringBuilder();
        for (int index = 0; index < value.length(); index++) {
            if (Character.isDigit(value.charAt(index))) normalized.append(value.charAt(index));
        }
        return normalized.toString();
    }

    private void appendNuinuiWebConsent(Dialog dialog, String query, LinearLayout transcript,
            ScrollView transcriptScroll) {
        if (query.isEmpty() || nuinuiQueryContainsPrivateActivity(query)) {
            appendChatBubble(transcript, l("No enviaré números, nombres ni datos de llamadas a la web. "
                            + "Formula una pregunta general sin datos privados.",
                    "I will not send numbers, names, or call data to the web. "
                            + "Ask a general question without private data."), false);
            return;
        }
        LinearLayout card = nuinuiInlineCard();
        card.addView(text(l("Consulta web opcional", "Optional web lookup"),
                14, colorText, Typeface.BOLD));
        TextView disclosure = text(l("Se enviará únicamente esta pregunta a Wikipedia. "
                        + "Nunca se adjuntan contactos, números ni historial.",
                "Only this question will be sent to Wikipedia. Contacts, numbers, "
                        + "and history are never attached."), 12, colorMuted, Typeface.NORMAL);
        disclosure.setPadding(0, dp(5), 0, dp(8));
        card.addView(disclosure);
        TextView exact = text(query, 13, colorText, Typeface.BOLD);
        exact.setPadding(dp(12), dp(10), dp(12), dp(10));
        exact.setBackground(rounded(dark ? Color.rgb(35, 47, 41) : Color.WHITE,
                14, colorBorder, 1));
        card.addView(exact);
        LinearLayout actions = new LinearLayout(this);
        actions.setOrientation(LinearLayout.HORIZONTAL);
        actions.setPadding(0, dp(10), 0, 0);
        TextView cancel = actionButton(l("Cancelar", "Cancel"), false,
                view -> card.setVisibility(View.GONE));
        actions.addView(cancel, new LinearLayout.LayoutParams(0, dp(48), 1f));
        TextView search = actionButton(l("Buscar", "Search"), true, null);
        LinearLayout.LayoutParams searchParams = new LinearLayout.LayoutParams(0, dp(48), 1f);
        searchParams.leftMargin = dp(8);
        actions.addView(search, searchParams);
        search.setOnClickListener(view -> {
            search.setEnabled(false);
            card.setVisibility(View.GONE);
            appendChatBubble(transcript, l("Buscando una fuente pública…",
                    "Searching a public source…"), false);
            scrollTranscript(transcriptScroll);
            NuinuiWebSearch.search(query, activeLanguageCode(), result -> runOnUiThread(() -> {
                if (!dialog.isShowing()) return;
                if (!result.success) {
                    appendChatBubble(transcript, nuinuiWebError(result.error), false);
                } else {
                    appendChatBubble(transcript, result.title + "\n\n" + result.summary
                            + "\n\nWikipedia · CC BY-SA", false);
                    TextView source = actionButton(l("Abrir fuente", "Open source"), false,
                            sourceView -> openNuinuiSource(result.sourceUrl));
                    LinearLayout.LayoutParams sourceParams = new LinearLayout.LayoutParams(
                            ViewGroup.LayoutParams.WRAP_CONTENT, dp(46));
                    sourceParams.leftMargin = dp(36);
                    sourceParams.topMargin = dp(6);
                    transcript.addView(source, sourceParams);
                }
                scrollTranscript(transcriptScroll);
            }));
        });
        card.addView(actions);
        addNuinuiInlineCard(transcript, card);
    }

    private String cleanNuinuiWebQuery(String raw) {
        String query = raw == null ? "" : raw.trim();
        String lower = query.toLowerCase(Locale.ROOT);
        String[] prefixes = {
                "buscar en web:", "search the web:",
                "buscar en web sobre telefonía:", "buscar en web sobre telefonia:",
                "search the web about telephony:"
        };
        for (String prefix : prefixes) {
            if (lower.startsWith(prefix)) return query.substring(prefix.length()).trim();
        }
        return query;
    }

    private boolean nuinuiQueryContainsPrivateActivity(String query) {
        if (digits(query).length() >= 7) return true;
        String lower = query.toLowerCase(Locale.ROOT);
        for (DecisionEvent event : loadActivityEvents()) {
            String name = activityDisplayName(event).toLowerCase(Locale.ROOT);
            if (name.length() >= 3 && lower.contains(name)) return true;
        }
        return false;
    }

    private String nuinuiWebError(String error) {
        if ("PRIVATE_OR_INVALID_QUERY".equals(error)) {
            return l("La consulta parece incluir datos de una llamada y no se enviará.",
                    "The query appears to contain call data and will not be sent.");
        }
        if ("NO_RESULT".equals(error) || "INCOMPLETE_RESULT".equals(error)) {
            return l("No encontré una fuente suficientemente clara. Prueba otra formulación.",
                    "I did not find a sufficiently clear source. Try rephrasing.");
        }
        return l("La búsqueda web no está disponible ahora. Nuinui sigue funcionando sin conexión.",
                "Web search is unavailable right now. Nuinui still works offline.");
    }

    private void openNuinuiSource(String sourceUrl) {
        if (sourceUrl == null || !sourceUrl.startsWith("https://")) return;
        try {
            startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(sourceUrl)));
        } catch (Exception error) {
            toast(l("No se pudo abrir la fuente", "The source could not be opened"));
        }
    }

    private LinearLayout nuinuiInlineCard() {
        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setPadding(dp(14), dp(13), dp(14), dp(13));
        card.setBackground(rounded(dark ? Color.rgb(27, 38, 33) : Color.rgb(245, 248, 246),
                18, colorBorder, 1));
        return card;
    }

    private void addNuinuiInlineCard(LinearLayout transcript, LinearLayout card) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.leftMargin = dp(36);
        params.topMargin = dp(7);
        transcript.addView(card, params);
    }

    private void scrollTranscript(ScrollView transcriptScroll) {
        transcriptScroll.post(() -> transcriptScroll.fullScroll(View.FOCUS_DOWN));
    }

    private String assistantText(String response) {
        String intent = field(response, "INTENT", "UNKNOWN");
        if ("WELCOME".equals(intent)) {
            return l("Hola, soy Nuinui, ¿qué deseas?",
                    "Hi, I’m Nuinui. What would you like?");
        }
        if ("STATUS".equals(intent)) {
            String state = "ACTIVE".equals(field(response, "PROTECTION", "ACTIVE"))
                    ? l("Activa", "Active") : l("Pausada", "Paused");
            return l("Protección", "Protection") + ": " + state + "\n"
                    + l("Bloqueadas", "Blocked") + ": " + field(response, "BLOCKED", "0")
                    + " · " + l("Permitidas", "Allowed") + ": "
                    + field(response, "ALLOWED", "0");
        }
        if ("SPAM_BASE".equals(intent)) {
            return l("Base antispam firmada", "Signed antispam base") + ": "
                    + ("ON".equals(field(response, "VERIFIED", "OFF"))
                    ? l("Verificada", "Verified") : l("No disponible", "Unavailable"))
                    + "\n" + l("Versión", "Version") + ": "
                    + field(response, "VERSION", "NONE") + " · "
                    + l("Indicadores", "Indicators") + ": "
                    + field(response, "INDICATORS", "0");
        }
        if ("RULES".equals(intent)) {
            return l("Reglas", "Rules") + " · " + l("Bloqueadas", "Blocked") + ": "
                    + field(response, "BLOCKED", "0") + " · "
                    + l("Permitidas", "Allowed") + ": " + field(response, "ALLOWED", "0");
        }
        if ("LEARNING".equals(intent)) {
            return l("Aprendizaje local", "Local learning") + ": "
                    + ("ON".equals(field(response, "ENABLED", "OFF"))
                    ? l("Activo", "Active") : l("Desactivado", "Disabled"))
                    + "\n" + l("Confianza", "Confidence") + ": "
                    + field(response, "CONFIDENCE", "0") + "% · "
                    + l("Muestras", "Samples") + ": " + field(response, "SAMPLES", "0");
        }
        if ("PRIVACY".equals(intent)) {
            return l("Nuinui funciona de forma local por defecto y solo lee la actividad "
                            + "cifrada creada por UnCall. Una búsqueda web requiere tu "
                            + "confirmación y nunca incluye números, nombres ni historial.",
                    "Nuinui works locally by default and reads only encrypted activity "
                            + "created by UnCall. A web lookup requires your confirmation and "
                            + "never includes numbers, names, or history.");
        }
        if ("LAST_DECISION".equals(intent)) {
            return l("Última decisión", "Last decision") + ": "
                    + field(response, "RESULT", "NONE") + "\n"
                    + field(response, "NUMBER", "") + " · "
                    + reasonLabel(field(response, "REASON", "NONE"), "");
        }
        if ("NO_ACTIVITY".equals(intent)) {
            return l("Todavía no hay llamadas evaluadas.", "No calls have been evaluated yet.");
        }
        if ("PROPOSE_RULE".equals(intent)) {
            return l("He preparado este cambio. Revísalo antes de aplicarlo.",
                    "I prepared this change. Review it before applying it.");
        }
        if ("NUMBER_LOOKUP".equals(intent)) {
            return l("Consulta local", "Local lookup") + " · "
                    + field(response, "NUMBER", "") + "\n"
                    + l("Riesgo", "Risk") + ": " + field(response, "RISK", "0")
                    + " · " + l("Acción", "Action") + ": "
                    + field(response, "ACTION", "NONE");
        }
        if ("INVALID_NUMBER".equals(intent) || "INVALID_RULE".equals(intent)) {
            return l("No puedo interpretar ese número o patrón. Usa una acción de reglas "
                            + "y vuelve a introducirlo.",
                    "I cannot interpret that number or pattern. Choose a rule action and "
                            + "enter it again.");
        }
        if ("WEB_TELEPHONY".equals(intent) || "WEB_GENERAL".equals(intent)) {
            return l("Puedo buscar una fuente pública sobre esa cuestión. Antes te mostraré "
                            + "exactamente qué texto saldrá del dispositivo.",
                    "I can search a public source about that topic. First I will show you "
                            + "exactly what text will leave the device.");
        }
        return l("Puedo ayudarte con llamadas evaluadas, reglas, antispam, privacidad y "
                        + "consultas generales de telefonía. Prueba una sugerencia o escribe "
                        + "una petición concreta.",
                "I can help with evaluated calls, rules, antispam, privacy, and general "
                        + "telephony questions. Try a suggestion or write a specific request.");
    }

    private LinearLayout dialogSheet() {
        LinearLayout sheet = new LinearLayout(this);
        sheet.setOrientation(LinearLayout.VERTICAL);
        sheet.setPadding(dp(24), dp(24), dp(24), dp(22));
        sheet.setBackground(glass(30, false));
        return sheet;
    }

    private LinearLayout dialogActions() {
        LinearLayout actions = new LinearLayout(this);
        actions.setOrientation(LinearLayout.HORIZONTAL);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.topMargin = dp(20);
        actions.setLayoutParams(params);
        return actions;
    }

    private void showBottomDialog(Dialog dialog, View content) {
        dialog.setContentView(content);
        dialog.setCanceledOnTouchOutside(true);
        dialog.show();
        Window window = dialog.getWindow();
        if (window == null) return;
        window.setBackgroundDrawableResource(android.R.color.transparent);
        window.setDimAmount(dark ? 0.36f : 0.24f);
        window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
        if (Build.VERSION.SDK_INT >= 31) {
            WindowManager.LayoutParams attributes = window.getAttributes();
            attributes.setBlurBehindRadius(dp(20));
            window.setAttributes(attributes);
            window.addFlags(WindowManager.LayoutParams.FLAG_BLUR_BEHIND);
        }
        window.setGravity(Gravity.BOTTOM);
        window.getDecorView().setPadding(dp(14), 0, dp(14), dp(14));
        int dialogWidth = Math.min(getResources().getDisplayMetrics().widthPixels,
                dp(tabletContent ? 620 : 720));
        window.setLayout(dialogWidth, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private void showInlineError(TextView error, String message) {
        error.setText(message);
        error.setVisibility(View.VISIBLE);
        error.announceForAccessibility(message);
    }

    private String normalizeRuleInput(String raw, boolean pattern) {
        String value = raw == null ? "" : raw.trim();
        if (!pattern) value = value.replace(" ", "").replace("-", "");
        if (value.contains("|") || value.contains("@")) return "";
        return value;
    }

    private View switchSetting(String title, String subtitle, boolean checked,
            android.widget.CompoundButton.OnCheckedChangeListener listener) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(8), 0, dp(8));
        LinearLayout words = new LinearLayout(this);
        words.setOrientation(LinearLayout.VERTICAL);
        words.addView(text(title, 15, colorText, Typeface.BOLD));
        words.addView(text(subtitle, 13, colorMuted, Typeface.NORMAL));
        row.addView(words, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        Switch toggle = new Switch(this);
        toggle.setMinimumWidth(dp(48));
        toggle.setMinimumHeight(dp(48));
        toggle.setChecked(checked);
        toggle.setContentDescription(title);
        toggle.setThumbTintList(new ColorStateList(
                new int[][] {new int[] {android.R.attr.state_checked}, new int[] {}},
                new int[] {colorGreen, colorMuted}));
        toggle.setTrackTintList(new ColorStateList(
                new int[][] {new int[] {android.R.attr.state_checked}, new int[] {}},
                new int[] {withAlpha(colorGreen, 92), withAlpha(colorMuted, 62)}));
        toggle.setOnCheckedChangeListener((button, enabled) -> {
            if (Boolean.TRUE.equals(button.getTag())) return;
            listener.onCheckedChanged(button, enabled);
        });
        row.addView(toggle);
        return row;
    }

    private View choiceSetting(String title, String subtitle, String[] labels,
            String[] values, String selected, ValueListener listener) {
        LinearLayout block = new LinearLayout(this);
        block.setOrientation(LinearLayout.VERTICAL);
        block.setPadding(0, dp(8), 0, dp(8));
        block.addView(text(title, 15, colorText, Typeface.BOLD));
        TextView detail = text(subtitle, 13, colorMuted, Typeface.NORMAL);
        detail.setPadding(0, dp(3), 0, dp(12));
        block.addView(detail);
        LinearLayout choices = new LinearLayout(this);
        choices.setOrientation(compactContent ? LinearLayout.VERTICAL : LinearLayout.HORIZONTAL);
        for (int i = 0; i < labels.length; i++) {
            final String value = values[i];
            boolean active = value.equals(selected);
            TextView option = text(labels[i], 12,
                    active ? colorGreenStrong : colorMuted, Typeface.BOLD);
            option.setGravity(Gravity.CENTER);
            option.setPadding(dp(8), dp(10), dp(8), dp(10));
            option.setBackground(navBackground(active));
            option.setClickable(true);
            option.setFocusable(true);
            option.setContentDescription(title + ": " + labels[i]);
            option.setOnClickListener(view -> listener.onValue(value));
            option.setMaxLines(2);
            if (Build.VERSION.SDK_INT >= 26 && !compactContent) {
                option.setAutoSizeTextTypeUniformWithConfiguration(10, 12, 1,
                        TypedValue.COMPLEX_UNIT_SP);
            }
            installPressFeedback(option);
            LinearLayout.LayoutParams params = compactContent
                    ? new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.WRAP_CONTENT)
                    : new LinearLayout.LayoutParams(0,
                            ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
            if (i > 0) {
                if (compactContent) params.topMargin = dp(7);
                else params.leftMargin = dp(7);
            }
            choices.addView(option, params);
        }
        block.addView(choices);
        return block;
    }

    private View actionRow(String title, String subtitle, String action, View.OnClickListener listener) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(compactContent ? LinearLayout.VERTICAL : LinearLayout.HORIZONTAL);
        row.setGravity(compactContent ? Gravity.START : Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(8), 0, dp(8));
        LinearLayout words = new LinearLayout(this);
        words.setOrientation(LinearLayout.VERTICAL);
        words.addView(text(title, 15, colorText, Typeface.BOLD));
        words.addView(text(subtitle, 13, colorMuted, Typeface.NORMAL));
        row.addView(words, compactContent
                ? new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT)
                : new LinearLayout.LayoutParams(0,
                        ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView button = actionButton(action, false, listener);
        if (compactContent) {
            LinearLayout.LayoutParams buttonParams = new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            buttonParams.topMargin = dp(10);
            row.addView(button, buttonParams);
        } else {
            row.addView(button);
        }
        return row;
    }

    private View labelValue(String label, String value, int valueColor) {
        if (compactContent || Translations.isRtl(activeLanguageCode())
                || label.length() + value.length() > 42) {
            return stackedLabelValue(label, value, valueColor);
        }
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(0, dp(9), 0, dp(9));
        row.addView(text(label, 14, colorText, Typeface.BOLD),
                new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView right = text(value, 13, valueColor, Typeface.NORMAL);
        right.setGravity(Gravity.END);
        row.addView(right);
        return row;
    }

    private View stackedLabelValue(String label, String value, int valueColor) {
        LinearLayout block = new LinearLayout(this);
        block.setOrientation(LinearLayout.VERTICAL);
        block.setPadding(0, dp(9), 0, dp(9));
        block.addView(text(label, 14, colorText, Typeface.BOLD));
        TextView detail = text(value, 13, valueColor, Typeface.NORMAL);
        detail.setPadding(0, dp(4), 0, 0);
        detail.setMaxLines(3);
        block.addView(detail, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return block;
    }

    private TextView actionButton(String label, boolean primary, View.OnClickListener action) {
        TextView button = text(label, 13, primary ? Color.WHITE : colorGreenStrong, Typeface.BOLD);
        button.setGravity(Gravity.CENTER);
        button.setPadding(dp(15), dp(10), dp(15), dp(10));
        int normal = primary ? colorGreen : colorGreenSoft;
        int pressed = primary ? colorGreenStrong
                : (dark ? Color.rgb(31, 77, 52) : Color.rgb(215, 247, 226));
        button.setBackground(pressable(normal, pressed, 16,
                primary ? Color.TRANSPARENT : withAlpha(colorGreen, 52), primary ? 0 : 1));
        button.setClickable(true);
        button.setFocusable(true);
        button.setMinimumHeight(dp(48));
        button.setMinWidth(dp(48));
        button.setOnClickListener(action);
        exposeAsButton(button);
        button.setMaxLines(2);
        if (Build.VERSION.SDK_INT >= 26 && !compactContent) {
            button.setAutoSizeTextTypeUniformWithConfiguration(10, 13, 1,
                    TypedValue.COMPLEX_UNIT_SP);
        }
        installPressFeedback(button);
        return button;
    }

    private TextView pill(String value, int background, int foreground) {
        TextView pill = text(value, 12, foreground, Typeface.BOLD);
        pill.setGravity(Gravity.CENTER);
        pill.setPadding(dp(11), dp(7), dp(11), dp(7));
        pill.setBackground(rounded(background, 18, Color.TRANSPARENT, 0));
        return pill;
    }

    private TextView sectionLabel(String value) {
        TextView label = text(value, 14, colorMuted, Typeface.BOLD);
        label.setPadding(dp(2), dp(8), 0, dp(10));
        markHeading(label);
        return label;
    }

    private LinearLayout pageBody() {
        LinearLayout body = new LinearLayout(this);
        body.setOrientation(LinearLayout.VERTICAL);
        int horizontal = tabletContent ? 24 : 18;
        int top = systemFontScale >= 1.60f ? 82 : 28;
        body.setPadding(dp(horizontal), dp(top), dp(horizontal), dp(104));
        return body;
    }

    private ScrollView scroll(View body) {
        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.setClipToPadding(false);
        FrameLayout contentLane = new FrameLayout(this);
        int available = getResources().getDisplayMetrics().widthPixels;
        int laneWidth = tabletContent ? Math.min(available, dp(760)) : available;
        FrameLayout.LayoutParams bodyParams = new FrameLayout.LayoutParams(
                laneWidth, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        contentLane.addView(body, bodyParams);
        scroll.addView(contentLane, new ScrollView.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        lastScrollY = 0;
        if (Build.VERSION.SDK_INT >= 23) {
            scroll.setOnScrollChangeListener((view, x, y, oldX, oldY) -> {
                updateTopGuard(y);
                int delta = y - lastScrollY;
                if (y <= dp(8)) showNavigation(false);
                else if (delta > dp(3)) showNavigation(true);
                else if (delta < -dp(3)) showNavigation(false);
                lastScrollY = y;
            });
        }
        return scroll;
    }

    private void updateTopGuard(int scrollY) {
        if (topGuard == null) return;
        if (currentTab != TAB_SETTINGS || scrollY <= 0) {
            topGuard.setAlpha(0f);
            topGuard.setVisibility(View.INVISIBLE);
            return;
        }
        float alpha = Math.min(1f, scrollY / (float) Math.max(1, dp(28)));
        topGuard.setVisibility(View.VISIBLE);
        topGuard.setAlpha(alpha);
    }

    private void showNavigation(boolean hidden) {
        if (navigationSuppressed || currentTab == TAB_SETTINGS) return;
        if (bottomBar == null || navigationHidden == hidden) return;
        navigationHidden = hidden;
        bottomBar.animate().cancel();
        if (hidden) {
            setNavigationInteractive(false);
            bottomBar.animate()
                    .translationY(dp(24))
                    .alpha(0f)
                    .scaleX(0.985f)
                    .scaleY(0.985f)
                    .setInterpolator(new PathInterpolator(0.40f, 0f, 1f, 1f))
                    .setDuration(230L)
                    .withEndAction(() -> {
                        if (navigationHidden && !navigationSuppressed) {
                            bottomBar.setVisibility(View.INVISIBLE);
                        }
                    })
                    .start();
            return;
        }
        bottomBar.setVisibility(View.VISIBLE);
        setNavigationInteractive(true);
        bottomBar.animate()
                .translationY(0f)
                .alpha(1f)
                .scaleX(1f)
                .scaleY(1f)
                .setInterpolator(new PathInterpolator(0.22f, 1f, 0.36f, 1f))
                .setDuration(310L)
                .start();
    }

    private void setNavigationInteractive(boolean interactive) {
        bottomBar.setClickable(interactive);
        bottomBar.setFocusable(interactive);
        bottomBar.setImportantForAccessibility(interactive
                ? View.IMPORTANT_FOR_ACCESSIBILITY_AUTO
                : View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
        for (ImageView item : navItems) {
            if (item == null) continue;
            item.setClickable(interactive);
            item.setFocusable(interactive);
        }
    }

    private void updateNavigationForTab(int tab) {
        if (bottomBar == null) return;
        bottomBar.animate().cancel();
        if (tab == TAB_SETTINGS) {
            navigationSuppressed = true;
            navigationHidden = true;
            setNavigationInteractive(false);
            bottomBar.animate()
                    .translationY(dp(22))
                    .alpha(0f)
                    .scaleX(0.985f)
                    .scaleY(0.985f)
                    .setInterpolator(new PathInterpolator(0.40f, 0f, 1f, 1f))
                    .setDuration(230L)
                    .withEndAction(() -> {
                        if (currentTab == TAB_SETTINGS) bottomBar.setVisibility(View.INVISIBLE);
                    })
                    .start();
            return;
        }
        boolean wasSuppressed = navigationSuppressed
                || bottomBar.getVisibility() != View.VISIBLE;
        navigationSuppressed = false;
        navigationHidden = false;
        bottomBar.setVisibility(View.VISIBLE);
        setNavigationInteractive(true);
        if (wasSuppressed) {
            bottomBar.setAlpha(0f);
            bottomBar.setTranslationY(dp(24));
            bottomBar.setScaleX(0.985f);
            bottomBar.setScaleY(0.985f);
        }
        bottomBar.animate()
                .translationY(0f)
                .alpha(1f)
                .scaleX(1f)
                .scaleY(1f)
                .setInterpolator(new PathInterpolator(0.22f, 1f, 0.36f, 1f))
                .setDuration(wasSuppressed ? 310L : 220L)
                .start();
    }

    private LinearLayout card() {
        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setPadding(dp(18), dp(17), dp(18), dp(17));
        card.setBackground(glass(22, false));
        card.setElevation(0f);
        return card;
    }

    private void addCard(LinearLayout parent, View card) {
        addWithBottom(parent, card, 14);
    }

    private void addWithBottom(LinearLayout parent, View child, int marginBottom) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.bottomMargin = dp(marginBottom);
        parent.addView(child, params);
    }

    private void addWithTop(LinearLayout parent, View child, int marginTop) {
        LinearLayout.LayoutParams params = wrapParams();
        params.topMargin = dp(marginTop);
        parent.addView(child, params);
    }

    private LinearLayout.LayoutParams wrapParams() {
        return new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private View divider() {
        View line = new View(this);
        line.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        line.setBackgroundColor(colorBorder);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(1));
        params.topMargin = dp(7);
        params.bottomMargin = dp(7);
        line.setLayoutParams(params);
        return line;
    }

    private View space(int height) {
        View space = new View(this);
        space.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        space.setLayoutParams(new LinearLayout.LayoutParams(1, dp(height)));
        return space;
    }

    private View emptyState(String title, String subtitle) {
        LinearLayout empty = new LinearLayout(this);
        empty.setOrientation(LinearLayout.VERTICAL);
        empty.setGravity(Gravity.CENTER);
        empty.setPadding(dp(8), dp(30), dp(8), dp(30));
        TextView heading = text(title, 16, colorText, Typeface.BOLD);
        markHeading(heading);
        empty.addView(heading);
        TextView sub = text(subtitle, 13, colorMuted, Typeface.NORMAL);
        sub.setGravity(Gravity.CENTER);
        sub.setPadding(0, dp(6), 0, 0);
        empty.addView(sub);
        return empty;
    }

    private TextView text(String value, int size, int color, int style) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(size);
        view.setTextColor(color);
        view.setTypeface(Typeface.create("sans-serif", style));
        view.setLineSpacing(0f, 1.08f);
        if (Build.VERSION.SDK_INT >= 23) {
            view.setBreakStrategy(android.text.Layout.BREAK_STRATEGY_HIGH_QUALITY);
            view.setHyphenationFrequency(android.text.Layout.HYPHENATION_FREQUENCY_NORMAL);
        }
        applyTextDirection(view);
        return view;
    }

    private void markHeading(TextView view) {
        if (Build.VERSION.SDK_INT >= 28) view.setAccessibilityHeading(true);
    }

    private void exposeAsButton(View view) {
        view.setAccessibilityDelegate(new View.AccessibilityDelegate() {
            @Override public void onInitializeAccessibilityNodeInfo(
                    View host, AccessibilityNodeInfo info) {
                super.onInitializeAccessibilityNodeInfo(host, info);
                info.setClassName(android.widget.Button.class.getName());
                info.setClickable(host.isClickable());
                info.setSelected(host.isSelected());
            }
        });
    }

    private void exposeExpandable(View view, String description, boolean expanded) {
        view.setContentDescription(description);
        view.setAccessibilityDelegate(new View.AccessibilityDelegate() {
            @Override public void onInitializeAccessibilityNodeInfo(
                    View host, AccessibilityNodeInfo info) {
                super.onInitializeAccessibilityNodeInfo(host, info);
                info.setClassName(android.widget.Button.class.getName());
                info.setClickable(true);
                if (Build.VERSION.SDK_INT >= 21) {
                    info.addAction(expanded
                            ? AccessibilityNodeInfo.AccessibilityAction.ACTION_COLLAPSE
                            : AccessibilityNodeInfo.AccessibilityAction.ACTION_EXPAND);
                }
            }
        });
    }

    private long motionDuration(long normalDuration) {
        return reducedMotion ? 0L : normalDuration;
    }

    private void applyTextDirection(TextView view) {
        view.setTextDirection(Translations.isRtl(activeLanguageCode())
                ? View.TEXT_DIRECTION_RTL : View.TEXT_DIRECTION_LTR);
    }

    private GradientDrawable rounded(int fill, int radiusDp, int stroke, int strokeDp) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(fill);
        drawable.setCornerRadius(dp(radiusDp));
        if (strokeDp > 0) drawable.setStroke(dp(strokeDp), stroke);
        return drawable;
    }

    private GradientDrawable glass(int radiusDp, boolean navigation) {
        if (navigation) {
            int top = dark ? Color.argb(222, 34, 45, 40)
                    : Color.argb(222, 242, 245, 243);
            int bottom = dark ? Color.argb(212, 23, 31, 27)
                    : Color.argb(216, 231, 236, 233);
            int edge = dark ? Color.argb(38, 255, 255, 255)
                    : Color.argb(32, 78, 95, 86);
            GradientDrawable bevel = new GradientDrawable(
                    GradientDrawable.Orientation.TOP_BOTTOM, new int[] {top, bottom});
            bevel.setCornerRadius(dp(radiusDp));
            bevel.setStroke(dp(1), edge);
            return bevel;
        }
        int fill = dark
                ? Color.argb(214, 31, 42, 37)
                : Color.argb(232, 255, 255, 255);
        int highlight = dark ? Color.argb(42, 255, 255, 255)
                : Color.argb(38, 15, 156, 73);
        return rounded(fill, radiusDp, highlight, 1);
    }

    private GradientDrawable settingsMenuBackground(boolean active) {
        if (active) return rounded(colorGreenSoft, 19, withAlpha(colorGreen, 72), 1);
        return glass(19, true);
    }

    private GradientDrawable topGuardBackground() {
        int transparent = Color.argb(0, Color.red(colorBackground),
                Color.green(colorBackground), Color.blue(colorBackground));
        return new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM,
                new int[] {colorBackground, colorBackground, transparent});
    }

    private RippleDrawable settingsMenuRipple() {
        ColorStateList ripple = ColorStateList.valueOf(withAlpha(colorGreen, dark ? 72 : 54));
        GradientDrawable mask = rounded(Color.WHITE, 19, Color.TRANSPARENT, 0);
        return new RippleDrawable(ripple, null, mask);
    }

    private StateListDrawable pressable(int normal, int pressed, int radiusDp,
            int stroke, int strokeDp) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[] {android.R.attr.state_pressed},
                rounded(pressed, radiusDp, stroke, strokeDp));
        states.addState(new int[] {android.R.attr.state_focused},
                rounded(pressed, radiusDp, stroke, strokeDp));
        states.addState(new int[] {}, rounded(normal, radiusDp, stroke, strokeDp));
        return states;
    }

    private StateListDrawable navBackground(boolean selected) {
        int normal = selected
                ? (dark ? Color.argb(124, 25, 64, 44) : Color.argb(150, 235, 249, 240))
                : Color.TRANSPARENT;
        int pressed = dark ? Color.argb(150, 35, 77, 55)
                : Color.argb(150, 218, 248, 229);
        return pressable(normal, pressed, 16,
                selected ? withAlpha(colorGreen, 32) : Color.TRANSPARENT, selected ? 1 : 0);
    }

    private void installPressFeedback(View view) {
        view.setOnTouchListener((target, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(0.84f);
                    return false;
                }
                target.animate().scaleX(0.97f).scaleY(0.97f).alpha(0.84f)
                        .setDuration(70L).start();
            } else if (event.getActionMasked() == MotionEvent.ACTION_UP
                    || event.getActionMasked() == MotionEvent.ACTION_CANCEL) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(1f);
                    return false;
                }
                target.animate().scaleX(1f).scaleY(1f).alpha(1f)
                        .setDuration(120L).start();
            }
            return false;
        });
    }

    private void installNavPressFeedback(ImageView view) {
        view.setOnTouchListener((target, event) -> {
            float restingScale = target.isSelected() ? 1.22f : 0.82f;
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(0.72f);
                    return false;
                }
                target.animate()
                        .scaleX(restingScale * 0.88f)
                        .scaleY(restingScale * 0.88f)
                        .alpha(0.72f)
                        .setDuration(65L)
                        .start();
            } else if (event.getActionMasked() == MotionEvent.ACTION_UP
                    || event.getActionMasked() == MotionEvent.ACTION_CANCEL) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(target.isSelected() ? 1f : 0.70f);
                    return false;
                }
                target.animate()
                        .scaleX(restingScale)
                        .scaleY(restingScale)
                        .alpha(target.isSelected() ? 1f : 0.70f)
                        .setInterpolator(new OvershootInterpolator(1.35f))
                        .setDuration(210L)
                        .start();
            }
            return false;
        });
    }

    private void installMenuPressFeedback(View view) {
        view.setOnTouchListener((target, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(0.70f);
                    return false;
                }
                target.animate().scaleX(0.88f).scaleY(0.88f).alpha(0.70f)
                        .translationY(dp(1)).setDuration(65L).start();
            } else if (event.getActionMasked() == MotionEvent.ACTION_UP
                    || event.getActionMasked() == MotionEvent.ACTION_CANCEL) {
                target.animate().cancel();
                if (reducedMotion) {
                    target.setAlpha(1f);
                    return false;
                }
                target.animate().scaleX(1f).scaleY(1f).alpha(1f).translationY(0f)
                        .setInterpolator(new OvershootInterpolator(1.7f))
                        .setDuration(230L).start();
            }
            return false;
        });
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private int withAlpha(int color, int alpha) {
        return Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color));
    }

    private String safeCommand(String command) {
        try {
            NativeBridge.ensureInitialized(this);
            return NativeBridge.command(command);
        } catch (Exception error) {
            return "ERR|" + error.getClass().getSimpleName();
        }
    }

    private void runMutation(String command, String successMessage) {
        String failure = mutationError(command);
        toast(failure == null ? successMessage : friendlyMutationError(failure));
        refreshCurrentPage();
    }

    private void runSwitchMutation(android.widget.CompoundButton button, boolean enabled,
            String command, String successMessage, Runnable onSuccess) {
        Object operation = new Object();
        button.setTag(operation);
        button.setClickable(false);
        button.animate().cancel();
        button.animate()
                .scaleX(1.07f)
                .scaleY(1.07f)
                .setDuration(90L)
                .withEndAction(() -> button.animate()
                        .scaleX(1f)
                        .scaleY(1f)
                        .setInterpolator(new OvershootInterpolator(1.15f))
                        .setDuration(210L)
                        .start())
                .start();

        new Thread(() -> {
            final String failure;
            synchronized (mutationLock) {
                failure = mutationError(command);
            }
            runOnUiThread(() -> {
                if (failure == null) {
                    if (onSuccess != null) onSuccess.run();
                    toast(successMessage);
                } else {
                    button.setTag(Boolean.TRUE);
                    button.setChecked(!enabled);
                    button.animate().cancel();
                    button.animate()
                            .scaleX(0.96f)
                            .scaleY(0.96f)
                            .setDuration(80L)
                            .withEndAction(() -> button.animate()
                                    .scaleX(1f)
                                    .scaleY(1f)
                                    .setInterpolator(new DecelerateInterpolator(1.7f))
                                    .setDuration(180L)
                                    .start())
                            .start();
                    toast(friendlyMutationError(failure));
                }
                if (button.getTag() == operation || Boolean.TRUE.equals(button.getTag())) {
                    button.setTag(null);
                }
                button.setClickable(true);
            });
        }, "UnCall-switch-mutation").start();
    }

    private String mutationError(String command) {
        try {
            NativeBridge.ensureInitialized(this);
            String response = NativeBridge.command(command);
            if (response == null || !response.startsWith("OK|")) {
                return response == null ? "ERR|EMPTY_RESPONSE" : response;
            }
            new BlocklistStore(this).save();
            return null;
        } catch (Exception error) {
            return "ERR|PERSISTENCE|" + error.getClass().getSimpleName();
        }
    }

    private String friendlyMutationError(String response) {
        if (response == null) return l("No se pudo guardar.", "Could not save.");
        if (response.contains("DUPLICATE")) {
            return l("La regla ya existe o está vacía.", "The rule already exists or is empty.");
        }
        if (response.contains("NOT_FOUND")) {
            return l("La regla ya no existe.", "The rule no longer exists.");
        }
        if (response.contains("PERSISTENCE")) {
            return l("No se pudo conservar el cambio en el dispositivo.",
                    "The change could not be persisted on the device.");
        }
        return l("El núcleo nC rechazó el cambio: ", "The nC core rejected the change: ")
                + response;
    }

    private String field(String response, String key, String fallback) {
        if (response == null || response.isEmpty()) return fallback;
        String[] fields = response.split("\\|", -1);
        for (int i = 0; i + 1 < fields.length; i++) {
            if (key.equals(fields[i])) return fields[i + 1];
        }
        return fallback;
    }

    private RuleGroups parseRules(String response) {
        RuleGroups groups = new RuleGroups();
        if (response == null || !response.startsWith("OK|")) return groups;
        String[] fields = response.split("\\|", -1);
        int index = 1;
        while (index + 1 < fields.length) {
            String group = fields[index++];
            int count;
            try { count = Integer.parseInt(fields[index++]); }
            catch (Exception error) { break; }
            List<String> target;
            if ("BLOCKLIST".equals(group)) target = groups.blocked;
            else if ("PATTERNS".equals(group) || "BLOCK_PATTERNS".equals(group)) {
                target = groups.blockPatterns;
            } else if ("WHITELIST".equals(group)) target = groups.allowed;
            else if ("ALLOW_PATTERNS".equals(group)) target = groups.allowPatterns;
            else target = null;
            for (int i = 0; i < count && index < fields.length; i++, index++) {
                if (target != null && !fields[index].isEmpty()) target.add(fields[index]);
            }
        }
        return groups;
    }

    private List<DecisionEvent> parseActivity(String response) {
        List<DecisionEvent> events = new ArrayList<>();
        if (response == null || !response.startsWith("OK|ACTIVITY|")) return events;
        String[] fields = response.split("\\|", -1);
        int index = 3;
        while (index < fields.length) {
            String head = fields[index++];
            int separator = head.indexOf('@');
            if (separator < 1 || index >= fields.length) break;
            long seconds;
            try { seconds = Long.parseLong(head.substring(0, separator)); }
            catch (Exception error) { seconds = 0L; }
            String result = head.substring(separator + 1);
            String reason = fields[index++];
            String rule = "";
            String number = "";
            if ("PATTERN".equals(reason) || "NUMBER".equals(reason)
                    || "ALLOW_PATTERN".equals(reason)) {
                if (index < fields.length) rule = fields[index++];
                if (index < fields.length) number = fields[index++];
            } else if (index < fields.length) {
                number = fields[index++];
            }
            events.add(new DecisionEvent("BLOCKED".equals(result), reason, rule,
                    number, seconds, 0L));
        }
        return events;
    }

    private List<DecisionEvent> loadActivityEvents() {
        List<ActivityStore.Record> stored = ActivityStore.load(this);
        if (stored.isEmpty()) return parseActivity(safeCommand("ACTIVIDAD"));
        List<DecisionEvent> events = new ArrayList<>();
        for (ActivityStore.Record record : stored) {
            DecisionEvent event = parseStoredDecision(record.decision, record.wallTime);
            if (event != null) events.add(event);
        }
        return events;
    }

    private DecisionEvent parseStoredDecision(String raw, long wallTime) {
        if (raw == null || raw.isEmpty()) return null;
        String[] fields = raw.split("\\|", -1);
        if (fields.length < 2) return null;
        boolean blocked = "BLOCKED".equals(fields[0]);
        String reason = fields[1];
        String rule = "";
        String number = "";
        if (("PATTERN".equals(reason) || "NUMBER".equals(reason)
                || "ALLOW_PATTERN".equals(reason)) && fields.length >= 4) {
            rule = fields[2];
            number = fields[3];
        } else if (fields.length >= 3) {
            number = fields[2];
        }
        return new DecisionEvent(blocked, reason, rule, number, 0L, wallTime);
    }

    private DecisionView formatDecision(String raw) {
        if (raw == null || raw.isEmpty() || "NONE".equals(raw)) {
            return new DecisionView(false, true,
                    l("Todavía no hay llamadas evaluadas.", "No calls have been evaluated yet."), "");
        }
        String[] fields = raw.split("\\|", -1);
        if (fields.length < 2) return new DecisionView(false, false, raw, "");
        boolean blocked = "BLOCKED".equals(fields[0]);
        String reason = fields[1];
        String rule = "";
        String number = "";
        if (("PATTERN".equals(reason) || "NUMBER".equals(reason)
                || "ALLOW_PATTERN".equals(reason)) && fields.length >= 4) {
            rule = fields[2];
            number = fields[3];
        } else if (fields.length >= 3) {
            number = fields[2];
        }
        return new DecisionView(blocked, false, reasonLabel(reason, rule),
                number.isEmpty() ? l("Número oculto", "Hidden number") : number);
    }

    private String reasonLabel(String reason, String rule) {
        if ("PATTERN".equals(reason)) return l("Patrón bloqueado: ", "Blocked pattern: ") + rule;
        if ("NUMBER".equals(reason)) return l("Número incluido en bloqueados", "Number is blocked");
        if ("ALLOW_PATTERN".equals(reason)) return l("Patrón permitido: ", "Allowed pattern: ") + rule;
        if ("HIDDEN_NUMBER".equals(reason)) return l("Número oculto", "Hidden number");
        if ("UNKNOWN_CONTACT".equals(reason)) return l("No está guardado en contactos", "Not saved in contacts");
        if ("URGENCY_FIRST".equals(reason)) return l("Primera llamada desconocida", "First unknown call");
        if ("CONTACT".equals(reason)) return l("Contacto guardado", "Saved contact");
        if ("WHITELIST".equals(reason)) return l("Número permitido manualmente", "Manually allowed number");
        if ("URGENCY_REPEAT".equals(reason)) return l("Segunda llamada de urgencia", "Second urgent call");
        if ("OPEN_MODE".equals(reason)) return l("Modo abierto", "Open mode");
        if ("PROTECTION_PAUSED".equals(reason)) return l("Protección pausada", "Protection paused");
        if ("INITIALIZATION_ERROR".equals(reason)) return l(
                "Permitida por error de inicialización", "Allowed due to initialization error");
        return reason.replace('_', ' ');
    }

    private String relativeTime(long seconds) {
        if (seconds <= 0L) return l("Reciente", "Recent");
        long age = SystemClock.elapsedRealtime() / 1000L - seconds;
        if (age < 0L || age < 5L) return l("Ahora", "Now");
        if (age < 60L) return Translations.relativeTime(activeLanguageCode(), age, 0);
        if (age < 3600L) {
            return Translations.relativeTime(activeLanguageCode(), age / 60L, 1);
        }
        return Translations.relativeTime(activeLanguageCode(), age / 3600L, 2);
    }

    private String relativeTime(DecisionEvent event) {
        if (event.wallTime <= 0L) return relativeTime(event.seconds);
        long age = Math.max(0L, (System.currentTimeMillis() - event.wallTime) / 1000L);
        if (age < 5L) return l("Ahora", "Now");
        if (age < 60L) return Translations.relativeTime(activeLanguageCode(), age, 0);
        if (age < 3600L) {
            return Translations.relativeTime(activeLanguageCode(), age / 60L, 1);
        }
        return Translations.relativeTime(activeLanguageCode(), age / 3600L, 2);
    }

    private String lastDecisionRaw() {
        String stored = prefs.getString(BlockMasterService.PREF_LAST_DECISION, "NONE");
        if (SecurityVault.isEncrypted(stored)) {
            try {
                return SecurityVault.decrypt(this, "last-decision", stored);
            } catch (Exception invalid) {
                return "NONE";
            }
        }
        if (!"NONE".equals(stored) && !stored.isEmpty()) {
            try {
                prefs.edit().putString(BlockMasterService.PREF_LAST_DECISION,
                        SecurityVault.encrypt(this, "last-decision", stored)).apply();
            } catch (Exception ignored) {}
        }
        return stored;
    }

    private long lastDecisionAt() {
        return prefs.getLong(BlockMasterService.PREF_LAST_DECISION_AT, 0L);
    }

    private boolean hasScreeningRole() {
        RoleManager manager = getSystemService(RoleManager.class);
        return manager != null && manager.isRoleHeld(RoleManager.ROLE_CALL_SCREENING);
    }

    private void reviewScreeningRole() {
        if (!hasScreeningRole()) {
            requestScreeningRole();
            return;
        }
        try {
            startActivity(new Intent(Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS));
        } catch (Exception unavailable) {
            openApplicationSettings();
        }
    }

    private void reviewContactsPermission() {
        if (checkSelfPermission(Manifest.permission.READ_CONTACTS)
                != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.READ_CONTACTS}, CONTACTS_REQUEST);
            return;
        }
        openApplicationSettings();
    }

    private void openApplicationSettings() {
        Intent intent = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.parse("package:" + getPackageName()));
        startActivity(intent);
    }

    private void requestScreeningRole() {
        RoleManager manager = getSystemService(RoleManager.class);
        if (manager != null && manager.isRoleAvailable(RoleManager.ROLE_CALL_SCREENING)) {
            startActivityForResult(manager.createRequestRoleIntent(RoleManager.ROLE_CALL_SCREENING),
                    ROLE_REQUEST);
        } else {
            startActivity(new Intent(Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS));
        }
    }

    private void requestMissingPermissions() {
        boolean notificationsMissing = BuildConfig.REMOTE_CONTROL_ENABLED
                && Build.VERSION.SDK_INT >= 33
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED;
        if (notificationsMissing) {
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS},
                    PERMISSIONS_REQUEST);
        }
    }

    @Override public void onRequestPermissionsResult(
            int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        refreshCurrentPage();
    }

    @Override protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        refreshCurrentPage();
    }

    private void ensureToken() throws Exception {
        if (!BuildConfig.REMOTE_CONTROL_ENABLED) return;
        SecurityVault.getOrCreateAccessToken(this, prefs);
    }

    private void copyToken() {
        if (!BuildConfig.REMOTE_CONTROL_ENABLED) return;
        try {
            String token = SecurityVault.getOrCreateAccessToken(this, prefs);
            ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
            ClipData clip = ClipData.newPlainText("UnCall token", token);
            if (Build.VERSION.SDK_INT >= 33) {
                PersistableBundle extras = new PersistableBundle();
                extras.putBoolean("android.content.extra.IS_SENSITIVE", true);
                clip.getDescription().setExtras(extras);
            }
            clipboard.setPrimaryClip(clip);
            new Handler(Looper.getMainLooper()).postDelayed(() -> {
                try {
                    ClipData current = clipboard.getPrimaryClip();
                    if (current != null && current.getItemCount() > 0
                            && token.contentEquals(current.getItemAt(0).coerceToText(this))) {
                        clipboard.clearPrimaryClip();
                    }
                } catch (Exception ignored) {}
            }, 60_000L);
            toast(l("Token copiado; se borrará del portapapeles en 60 segundos",
                    "Token copied; it will be cleared from the clipboard in 60 seconds"));
        } catch (Exception error) {
            toast(l("No se pudo abrir el almacén criptográfico",
                    "The cryptographic vault could not be opened"));
        }
    }

    private String maskToken(String token) {
        if (token == null || token.length() < 12) return l("Configurado", "Configured");
        return token.substring(0, 6) + " ········ " + token.substring(token.length() - 6);
    }

    private String l(String spanish, String english) {
        String mode = activeLanguageCode();
        if (LANGUAGE_ES.equals(mode)) return spanish;
        if (LANGUAGE_EN.equals(mode)) return english;
        return Translations.translate(mode, english);
    }

    private String activeLanguageCode() {
        String mode = prefs == null ? LANGUAGE_SYSTEM
                : prefs.getString(PREF_LANGUAGE_MODE, LANGUAGE_SYSTEM);
        return LANGUAGE_SYSTEM.equals(mode) ? Translations.systemLanguage() : mode;
    }

    private void applyLanguageDirection() {
        // Keep navigation, cards and gesture geometry stable in every language.
        // Arabic keeps its native reading order at TextView/EditText level.
        getWindow().getDecorView().setLayoutDirection(View.LAYOUT_DIRECTION_LTR);
    }

    private void showSecurityFailure() {
        LinearLayout failure = new LinearLayout(this);
        failure.setOrientation(LinearLayout.VERTICAL);
        failure.setGravity(Gravity.CENTER);
        failure.setPadding(dp(28), dp(28), dp(28), dp(28));
        failure.setBackgroundColor(colorBackground);
        failure.addView(text("UnCall", 30, colorText, Typeface.BOLD));
        TextView detail = text(l("La identidad o el almacén seguro de la aplicación no es válido. "
                        + "La protección no se iniciará.",
                "The application identity or secure vault is invalid. Protection will not start."),
                16, colorRed, Typeface.BOLD);
        detail.setGravity(Gravity.CENTER);
        detail.setPadding(0, dp(16), 0, 0);
        failure.addView(detail);
        setContentView(failure);
    }

    private void toast(String message) {
        Toast.makeText(this, message, Toast.LENGTH_LONG).show();
    }

    private static final class RuleGroups {
        final List<String> blocked = new ArrayList<>();
        final List<String> blockPatterns = new ArrayList<>();
        final List<String> allowed = new ArrayList<>();
        final List<String> allowPatterns = new ArrayList<>();
    }

    private static final class DecisionView {
        final boolean blocked;
        final boolean none;
        final String reason;
        final String number;

        DecisionView(boolean blocked, boolean none, String reason, String number) {
            this.blocked = blocked;
            this.none = none;
            this.reason = reason;
            this.number = number;
        }
    }

    private static final class DecisionEvent {
        final boolean blocked;
        final String reason;
        final String rule;
        final String number;
        final long seconds;
        final long wallTime;

        DecisionEvent(boolean blocked, String reason, String rule,
                String number, long seconds, long wallTime) {
            this.blocked = blocked;
            this.reason = reason;
            this.rule = rule;
            this.number = number;
            this.seconds = seconds;
            this.wallTime = wallTime;
        }
    }

    private static final class ActivityDay {
        final String key;
        final long wallTime;
        final List<DecisionEvent> blocked = new ArrayList<>();
        final List<DecisionEvent> allowed = new ArrayList<>();

        ActivityDay(String key, long wallTime) {
            this.key = key;
            this.wallTime = wallTime;
        }
    }

    private interface ValueListener {
        void onValue(String value);
    }
}
