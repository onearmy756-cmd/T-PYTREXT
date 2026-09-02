import React from 'react';
import { View, Platform, StyleSheet } from 'react-native';
import { WebView } from 'react-native-webview';

export default function App() {
  // During development: run a static file server serving repo root. On Android emulator use 10.0.2.2
  const uri = Platform.OS === 'android' ? 'http://10.0.2.2:8000/frontend/index.html' : 'http://localhost:8000/frontend/index.html';
  return (
    <View style={styles.container}>
      <WebView source={{ uri }} style={{ flex: 1 }} />
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 }
});
