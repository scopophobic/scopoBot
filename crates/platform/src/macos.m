#import <CoreAudio/CoreAudio.h>
#import <Foundation/Foundation.h>
#import <IOBluetooth/IOBluetooth.h>

// Returns only normalized device state; never names, identifiers, or audio samples.
int scopobot_audio(float *volume, int *headphones) {
    AudioDeviceID device = kAudioObjectUnknown;
    UInt32 size = sizeof(device);
    AudioObjectPropertyAddress address = { kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain };
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL, &size, &device) != noErr || device == kAudioObjectUnknown) return 0;
    address.mSelector = kAudioDevicePropertyVolumeScalar;
    address.mScope = kAudioDevicePropertyScopeOutput;
    Float32 scalar = 0;
    size = sizeof(scalar);
    if (AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &scalar) != noErr) {
        address.mElement = 1;
        if (AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &scalar) != noErr) scalar = -1;
    }
    UInt32 muted = 0;
    address.mSelector = kAudioDevicePropertyMute;
    address.mElement = kAudioObjectPropertyElementMain;
    size = sizeof(muted);
    AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &muted);
    *volume = scalar < 0 ? -1 : (muted ? 0 : scalar * 100);
    UInt32 transport = 0;
    address.mSelector = kAudioDevicePropertyTransportType;
    address.mScope = kAudioObjectPropertyScopeGlobal;
    size = sizeof(transport);
    AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &transport);
    // Bluetooth output may be a speaker; it is not sufficient to claim headphones.
    *headphones = -1;
    if (transport == kAudioDeviceTransportTypeBuiltIn) {
        UInt32 source = 0;
        address.mSelector = kAudioDevicePropertyDataSource;
        address.mScope = kAudioDevicePropertyScopeOutput;
        size = sizeof(source);
        if (AudioObjectGetPropertyData(device, &address, 0, NULL, &size, &source) == noErr)
            *headphones = source == 'hdpn' || source == 'hdph';
    }
    return 1;
}
int scopobot_bluetooth(void) {
    @autoreleasepool {
        for (IOBluetoothDevice *device in [IOBluetoothDevice pairedDevices]) {
            if ([device isConnected]) {
                uint32_t major = ([device classOfDevice] >> 8) & 0x1f;
                if (major == 4) return 1;
                if (major == 5) return 3;
                return 4;
            }
        }
    }
    return 0;
}

// Aggregate output-stream activity only. No taps, samples, app names, or PIDs.
int scopobot_playback(void) {
    NSOperatingSystemVersion version = [[NSProcessInfo processInfo] operatingSystemVersion];
    if (version.majorVersion > 14 || (version.majorVersion == 14 && version.minorVersion >= 2)) {
        AudioObjectPropertyAddress address = { kAudioHardwarePropertyProcessObjectList, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain };
        UInt32 size = 0;
        if (AudioObjectGetPropertyDataSize(kAudioObjectSystemObject, &address, 0, NULL, &size) != noErr || size > 4096 * sizeof(AudioObjectID)) return -1;
        if (size == 0) return 0;
        AudioObjectID *objects = calloc(1, size);
        if (!objects) return -1;
        if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL, &size, objects) != noErr) { free(objects); return -1; }
        address.mSelector = kAudioProcessPropertyIsRunningOutput;
        int result = -1;
        for (UInt32 index = 0; index < size / sizeof(AudioObjectID); index++) {
            UInt32 active = 0, bytes = sizeof(active);
            if (AudioObjectGetPropertyData(objects[index], &address, 0, NULL, &bytes, &active) == noErr) {
                result = 0;
                if (active) { result = 1; break; }
            }
        }
        free(objects);
        return result;
    }
    return -1;
}
