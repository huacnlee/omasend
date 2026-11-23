package com.longbridge.nsh;

import android.app.Activity;
import android.content.*;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import org.json.*;
import java.io.*;
import java.nio.file.Files;
import java.util.concurrent.Executors;

/** Native Android services alongside NSH's Rust UI. */
public final class OmasendActivity extends Activity {
    private static volatile OmasendActivity current;
    private NshView view;
    private long picking;
    private static native void initialize(String home, String cache);
    private static native void complete(long id, String json);

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        current = this;
        System.loadLibrary("nsh");
        initialize(getFilesDir().getAbsolutePath(), getCacheDir().getAbsolutePath());
        DevActivity.edgeToEdge(this);
        view = new NshView(this);
        view.useStandaloneLimits();
        setContentView(view);
        view.runRustApp("nsh");
    }
    public static void request(long id, int kind, String value) {
        OmasendActivity activity = current;
        if (activity == null) { error(id, "Activity unavailable"); return; }
        activity.runOnUiThread(() -> activity.perform(id, kind, value));
    }
    private void perform(long id, int kind, String value) {
        try {
            switch (kind) {
            case 0: case 5:
                if (picking != 0) throw new IOException("A picker is already open");
                Intent picker = new Intent(kind == 5 && android.os.Build.VERSION.SDK_INT >= 33
                    ? "android.provider.action.PICK_IMAGES" : Intent.ACTION_OPEN_DOCUMENT);
                if (kind == 5 && android.os.Build.VERSION.SDK_INT >= 33) {
                    picker.putExtra("android.provider.extra.PICK_IMAGES_MAX", Math.min(100, android.provider.MediaStore.getPickImagesMaxLimit()));
                } else {
                    picker.addCategory(Intent.CATEGORY_OPENABLE);
                    picker.setType("*/*");
                    if (kind == 5) picker.putExtra(Intent.EXTRA_MIME_TYPES, new String[]{"image/*", "video/*"});
                    picker.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                }
                picking = id;
                try { startActivityForResult(picker, 1); }
                catch (Exception e) { picking = 0; throw e; }
                break;
            case 1: throw new IOException("Choose files instead of folders on Android");
            case 2:
                ClipboardManager clipboard = (ClipboardManager)getSystemService(CLIPBOARD_SERVICE);
                ClipData clip = clipboard.getPrimaryClip();
                if (clip == null || clip.getItemCount() == 0) throw new IOException("Clipboard is empty");
                ClipData.Item item = clip.getItemAt(0);
                if (item.getUri() != null) importUris(id, new Uri[]{item.getUri()});
                else complete(id, new JSONObject().put("text", item.coerceToText(this)).toString());
                break;
            case 3:
                startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(value)));
                complete(id, "{}"); break;
            case 4:
                File source = new File(value);
                File root = new File(getCacheDir(), "share/" + java.util.UUID.randomUUID());
                if (!root.mkdirs()) throw new IOException("Cannot create share directory");
                File shared = new File(root, source.getName());
                Files.copy(source.toPath(), shared.toPath());
                Uri uri = new Uri.Builder().scheme("content").authority(getPackageName()+".files")
                    .appendPath(root.getName()).appendPath(shared.getName()).build();
                Intent intent = new Intent(Intent.ACTION_SEND).setType(OmasendFiles.mime(shared.getName()))
                    .putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                intent.setClipData(ClipData.newRawUri("File", uri));
                startActivity(Intent.createChooser(intent, null));
                complete(id, "{}"); break;
            default: throw new IOException("Unknown Android action");
            }
        } catch (Exception e) { error(id, e.getMessage()); }
    }
    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != 1 || picking == 0) return;
        long id = picking; picking = 0;
        if (result != RESULT_OK || data == null) { complete(id, "{\"files\":[]}"); return; }
        if (data.getClipData() != null) {
            ClipData clips = data.getClipData(); Uri[] uris = new Uri[clips.getItemCount()];
            for (int i=0; i<uris.length; i++) uris[i] = clips.getItemAt(i).getUri();
            importUris(id, uris);
        } else if (data.getData() != null) importUris(id, new Uri[]{data.getData()});
        else complete(id, "{\"files\":[]}");
    }
    private void importUris(long id, Uri[] uris) {
        java.util.concurrent.ExecutorService worker = Executors.newSingleThreadExecutor();
        worker.execute(() -> {
            try {
                JSONArray files = new JSONArray();
                File directory = new File(getCacheDir(), "imports/" + java.util.UUID.randomUUID());
                for (int i=0; i<uris.length; i++) {
                    String name = "file";
                    try (Cursor cursor = getContentResolver().query(uris[i], new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
                        if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) name = cursor.getString(0);
                    }
                    name = new File(name).getName();
                    if (name.equals(".") || name.equals("..") || name.isEmpty()) name = "file";
                    File parent = new File(directory, Integer.toString(i));
                    if (!parent.mkdirs()) throw new IOException("Cannot create import directory");
                    File target = new File(parent, name);
                    try (InputStream input = getContentResolver().openInputStream(uris[i])) {
                        if (input == null) throw new IOException("Cannot read selected file");
                        Files.copy(input, target.toPath());
                    }
                    files.put(target.getAbsolutePath());
                }
                complete(id, new JSONObject().put("files", files).toString());
            } catch (Exception e) { error(id, e.getMessage()); }
            finally { worker.shutdown(); }
        });
    }
    private static void error(long id, String message) {
        try { complete(id, new JSONObject().put("error", message == null ? "Android action failed" : message).toString()); }
        catch (JSONException e) { complete(id, "{\"error\":\"Android action failed\"}"); }
    }
    @Override public void onConfigurationChanged(android.content.res.Configuration config) {
        super.onConfigurationChanged(config); DevActivity.systemBars(this);
    }
    @Override @SuppressWarnings("deprecation") public void onBackPressed() {
        if (view == null || !view.handleBack()) super.onBackPressed();
    }
    @Override protected void onDestroy() {
        if (picking != 0) { complete(picking, "{\"files\":[]}"); picking = 0; }
        if (current == this) current = null;
        view.destroy(); super.onDestroy();
    }
}
