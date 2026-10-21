#import <UIKit/UIKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#import <PhotosUI/PhotosUI.h>

typedef void (*OmasendReply)(void *, const char *);

static UIViewController *presenter(void) {
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
        if (![scene isKindOfClass:UIWindowScene.class] || scene.activationState != UISceneActivationStateForegroundActive) continue;
        for (UIWindow *window in ((UIWindowScene *)scene).windows) {
            if (!window.isKeyWindow) continue;
            UIViewController *controller = window.rootViewController;
            while (controller.presentedViewController && !controller.presentedViewController.isBeingDismissed) controller = controller.presentedViewController;
            return controller;
        }
    }
    return nil;
}

static void reply(OmasendReply callback, void *context, NSDictionary *result) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
    NSString *json = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    callback(context, json.UTF8String);
}

@interface OmasendPicker : NSObject <UIDocumentPickerDelegate>
@property(nonatomic, copy) void (^completion)(NSDictionary *);
@end
static OmasendPicker *activePicker;

@implementation OmasendPicker
- (void)documentPicker:(UIDocumentPickerViewController *)controller didPickDocumentsAtURLs:(NSArray<NSURL *> *)urls {
    NSMutableArray *files = [NSMutableArray array];
    NSError *error = nil;
    // Import copies into an app-owned directory: the picker grants temporary access.
    NSURL *directory = [NSURL fileURLWithPath:[NSTemporaryDirectory() stringByAppendingPathComponent:NSUUID.UUID.UUIDString] isDirectory:YES];
    [NSFileManager.defaultManager createDirectoryAtURL:directory withIntermediateDirectories:YES attributes:nil error:&error];
    for (NSURL *url in urls) {
        if (error) break;
        BOOL scoped = [url startAccessingSecurityScopedResource];
        NSURL *destination = [directory URLByAppendingPathComponent:url.lastPathComponent];
        [NSFileManager.defaultManager copyItemAtURL:url toURL:destination error:&error];
        if (scoped) [url stopAccessingSecurityScopedResource];
        if (!error) [files addObject:destination.path];
    }
    self.completion(error ? @{ @"error": error.localizedDescription } : @{ @"files": files });
    activePicker = nil;
}
- (void)documentPickerWasCancelled:(UIDocumentPickerViewController *)controller {
    self.completion(@{ @"files": @[] });
    activePicker = nil;
}
@end

@interface OmasendPhotos : NSObject <PHPickerViewControllerDelegate>
@property(nonatomic, copy) void (^completion)(NSDictionary *);
@end
static OmasendPhotos *activePhotos;

@implementation OmasendPhotos
- (void)picker:(PHPickerViewController *)picker didFinishPicking:(NSArray<PHPickerResult *> *)results {
    [picker dismissViewControllerAnimated:YES completion:nil];
    if (results.count == 0) {
        self.completion(@{ @"files": @[] });
        activePhotos = nil;
        return;
    }
    // Item-provider URLs expire when their callback returns. Copy each original
    // into our sandbox there, retaining selection order even if loads finish out of order.
    dispatch_group_t group = dispatch_group_create();
    NSLock *lock = [NSLock new];
    NSMutableArray *paths = [NSMutableArray array];
    for (NSUInteger index = 0; index < results.count; index++) [paths addObject:NSNull.null];
    __block NSError *failure = nil;
    NSString *batch = [NSTemporaryDirectory() stringByAppendingPathComponent:NSUUID.UUID.UUIDString];
    [results enumerateObjectsUsingBlock:^(PHPickerResult *result, NSUInteger index, BOOL *stop) {
        (void)stop;
        NSItemProvider *provider = result.itemProvider;
        NSString *type = [provider hasItemConformingToTypeIdentifier:UTTypeMovie.identifier] ? UTTypeMovie.identifier : UTTypeImage.identifier;
        dispatch_group_enter(group);
        [provider loadFileRepresentationForTypeIdentifier:type completionHandler:^(NSURL *url, NSError *loadError) {
            NSError *error = loadError;
            NSString *path = nil;
            if (url && !error) {
                NSString *directory = [batch stringByAppendingPathComponent:[NSString stringWithFormat:@"%lu", (unsigned long)index]];
                [NSFileManager.defaultManager createDirectoryAtPath:directory withIntermediateDirectories:YES attributes:nil error:&error];
                NSString *name = provider.suggestedName.lastPathComponent;
                if (!name.length) name = url.lastPathComponent;
                if (!name.pathExtension.length && url.pathExtension.length) name = [name stringByAppendingPathExtension:url.pathExtension];
                path = [directory stringByAppendingPathComponent:name];
                if (!error) [NSFileManager.defaultManager copyItemAtURL:url toURL:[NSURL fileURLWithPath:path] error:&error];
            }
            if (!path && !error) error = [NSError errorWithDomain:@"Omasend" code:1 userInfo:@{NSLocalizedDescriptionKey: @"Could not import the selected photo or video"}];
            [lock lock];
            if (error) failure = error;
            else paths[index] = path;
            [lock unlock];
            dispatch_group_leave(group);
        }];
    }];
    dispatch_group_notify(group, dispatch_get_main_queue(), ^{
        if (failure) [NSFileManager.defaultManager removeItemAtPath:batch error:nil];
        self.completion(failure ? @{ @"error": failure.localizedDescription } : @{ @"files": paths });
        activePhotos = nil;
    });
}
@end

static void presentFiles(UIViewController *host, BOOL folders, void *context, OmasendReply callback) {
    if (activePicker || activePhotos) { reply(callback, context, @{ @"error": @"A picker is already open" }); return; }
    UIDocumentPickerViewController *picker = [[UIDocumentPickerViewController alloc] initForOpeningContentTypes:@[folders ? UTTypeFolder : UTTypeItem] asCopy:YES];
    picker.allowsMultipleSelection = YES;
    activePicker = [OmasendPicker new];
    activePicker.completion = ^(NSDictionary *result) { reply(callback, context, result); };
    picker.delegate = activePicker;
    [host presentViewController:picker animated:YES completion:nil];
}

static void presentPhotos(UIViewController *host, void *context, OmasendReply callback) {
    if (activePicker || activePhotos) { reply(callback, context, @{ @"error": @"A picker is already open" }); return; }
    PHPickerConfiguration *configuration = [PHPickerConfiguration new];
    configuration.selectionLimit = 0;
    configuration.selection = PHPickerConfigurationSelectionOrdered;
    configuration.filter = [PHPickerFilter anyFilterMatchingSubfilters:@[PHPickerFilter.imagesFilter, PHPickerFilter.videosFilter]];
    configuration.preferredAssetRepresentationMode = PHPickerConfigurationAssetRepresentationModeCurrent;
    PHPickerViewController *picker = [[PHPickerViewController alloc] initWithConfiguration:configuration];
    activePhotos = [OmasendPhotos new];
    activePhotos.completion = ^(NSDictionary *result) { reply(callback, context, result); };
    picker.delegate = activePhotos;
    [host presentViewController:picker animated:YES completion:nil];
}

// All UIKit work is dispatched to the main thread. The reply consumes the Rust
// request exactly once; its UTF-8 buffer is valid only during the callback.
void omasend_ios_request(int kind, const char *value, void *context, OmasendReply callback) {
    NSString *argument = value ? [NSString stringWithUTF8String:value] : @"";
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *host = presenter();
        if (!host) { reply(callback, context, @{ @"error": @"No active application window" }); return; }
        if (kind == 0 || kind == 1) {
            presentFiles(host, kind == 1, context, callback);
        } else if (kind == 5) {
            presentPhotos(host, context, callback);
        } else if (kind == 2) {
            UIPasteboard *board = UIPasteboard.generalPasteboard;
            UIImage *image = board.image;
            if (image) {
                NSString *path = [NSTemporaryDirectory() stringByAppendingPathComponent:[NSUUID.UUID.UUIDString stringByAppendingPathExtension:@"png"]];
                NSError *error = nil;
                [UIImagePNGRepresentation(image) writeToFile:path options:NSDataWritingAtomic error:&error];
                reply(callback, context, error ? @{ @"error": error.localizedDescription } : @{ @"files": @[path] });
            } else if (board.string.length) {
                reply(callback, context, @{ @"text": board.string });
            } else {
                reply(callback, context, @{ @"error": @"Clipboard has no image or text" });
            }
        } else if (kind == 3) {
            NSURL *url = [NSURL URLWithString:argument];
            if (!url) { reply(callback, context, @{ @"error": @"Invalid link" }); return; }
            [UIApplication.sharedApplication openURL:url options:@{} completionHandler:^(BOOL success) {
                reply(callback, context, success ? @{} : @{ @"error": @"Cannot open link" });
            }];
        } else {
            NSURL *url = [NSURL fileURLWithPath:argument];
            if (![NSFileManager.defaultManager fileExistsAtPath:url.path]) { reply(callback, context, @{ @"error": @"Received file no longer exists" }); return; }
            UIActivityViewController *share = [[UIActivityViewController alloc] initWithActivityItems:@[url] applicationActivities:nil];
            share.popoverPresentationController.sourceView = host.view;
            share.popoverPresentationController.sourceRect = CGRectMake(CGRectGetMidX(host.view.bounds), CGRectGetMidY(host.view.bounds), 1, 1);
            [host presentViewController:share animated:YES completion:nil];
            reply(callback, context, @{});
        }
    });
}
