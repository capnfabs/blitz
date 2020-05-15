import 'dart:typed_data';

import 'package:file_chooser/file_chooser.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:menubar/menubar.dart';
import 'blitz.dart' as blitz;

void main() {
  runApp(MyApp());
}

class MyApp extends StatefulWidget {
  @override
  _AppState createState() => new _AppState();
}

class _AppState extends State<MyApp> {
  Color _primaryColor = Colors.blue;
  Uint8List _image;
  String title = 'Flutter Demo Home Page';

  @override
  Widget build(BuildContext context) {
    // Any time the state changes, the menu needs to be rebuilt.
    updateMenubar();

    return MaterialApp(
      title: 'Flutter Demo',
      theme: ThemeData(
        primarySwatch: _primaryColor,
        // This makes the visual density adapt to the platform that you run
        // the app on. For desktop platforms, the controls will be smaller and
        // closer together (more dense) than on mobile platforms.
        visualDensity: VisualDensity.adaptivePlatformDensity,
      ),
      home: MyHomePage(
        title: title,
        image: _image,
        ),
    );
  }

  /// Sets the primary color of the app.
  void setPrimaryColor(Color color) {
    setState(() {
      _primaryColor = color;
    });
  }

  void setImage(Uint8List image) {
    setState(() {
      _image = image;
    });
  }

  void chooseFile() async {
    print("oh boy");
    final result = await showOpenPanel(allowedFileTypes: <FileTypeFilterGroup>[
      FileTypeFilterGroup(label: 'Fuji RAF', fileExtensions: <String>[
        'raf',
      ])
    ]);
    print(result.paths);
    final api = blitz.getApi();
    if (result.canceled) {
      return;
    }
    final renderer = api.newRenderer(result.paths[0]);
    final preview = api.loadPreview(renderer);
    print("HI JPEG");
    print(preview.length);
    print(preview.sublist(0,100));
    setImage(preview);
  }

  void updateMenubar() {
    setApplicationMenu([
      Submenu(label: 'File', children: [
        MenuItem(
            label: 'Open',
            shortcut:
                LogicalKeySet(LogicalKeyboardKey.meta, LogicalKeyboardKey.keyO),
            onClicked: () async {
              chooseFile();
            }),
        MenuDivider(),
      ]),
      Submenu(label: 'Color', children: [
        MenuItem(
            label: 'Reset',
            enabled: _primaryColor != Colors.blue,
            shortcut: LogicalKeySet(
                LogicalKeyboardKey.meta, LogicalKeyboardKey.backspace),
            onClicked: () {
              setPrimaryColor(Colors.blue);
            }),
        MenuDivider(),
        Submenu(label: 'Presets', children: [
          MenuItem(
              label: 'Red',
              enabled: _primaryColor != Colors.red,
              shortcut: LogicalKeySet(LogicalKeyboardKey.meta,
                  LogicalKeyboardKey.shift, LogicalKeyboardKey.keyR),
              onClicked: () {
                setPrimaryColor(Colors.red);
              }),
          MenuItem(
              label: 'Green',
              enabled: _primaryColor != Colors.green,
              shortcut: LogicalKeySet(LogicalKeyboardKey.meta,
                  LogicalKeyboardKey.alt, LogicalKeyboardKey.keyG),
              onClicked: () {
                setPrimaryColor(Colors.green);
              }),
          MenuItem(
              label: 'Purple',
              enabled: _primaryColor != Colors.deepPurple,
              shortcut: LogicalKeySet(LogicalKeyboardKey.meta,
                  LogicalKeyboardKey.control, LogicalKeyboardKey.keyP),
              onClicked: () {
                setPrimaryColor(Colors.deepPurple);
              }),
        ])
      ]),
    ]);
  }
}

class MyHomePage extends StatefulWidget {
  MyHomePage({Key key, this.title, this.image}) : super(key: key);

  final String title;
  final Uint8List image;

  @override
  _MyHomePageState createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  int _counter = 0;

  void _incrementCounter() {
    setState(() {
      _counter = blitz.getApi().addition(_counter, 2);
    });
  }

  @override
  Widget build(BuildContext context) {
    // This method is rerun every time setState is called, for instance as done
    // by the _incrementCounter method above.
    //
    // The Flutter framework has been optimized to make rerunning build methods
    // fast, so that you can just rebuild anything that needs updating rather
    // than having to individually change instances of widgets.
    return Scaffold(
      appBar: AppBar(
        // Here we take the value from the MyHomePage object that was created by
        // the App.build method, and use it to set our appbar title.
        title: Text(widget.title),
      ),
      body: Center(
        // Center is a layout widget. It takes a single child and positions it
        // in the middle of the parent.
        child: Column(
          // Column is also a layout widget. It takes a list of children and
          // arranges them vertically. By default, it sizes itself to fit its
          // children horizontally, and tries to be as tall as its parent.
          //
          // Invoke "debug painting" (press "p" in the console, choose the
          // "Toggle Debug Paint" action from the Flutter Inspector in Android
          // Studio, or the "Toggle Debug Paint" command in Visual Studio Code)
          // to see the wireframe for each widget.
          //
          // Column has various properties to control how it sizes itself and
          // how it positions its children. Here we use mainAxisAlignment to
          // center the children vertically; the main axis here is the vertical
          // axis because Columns are vertical (the cross axis would be
          // horizontal).
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            Text(
              'You have pushed the button this many times:',
            ),
            Text(
              '$_counter',
              style: Theme.of(context).textTheme.headline4,
            ),
            Image.memory(widget.image),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _incrementCounter,
        tooltip: 'Increment',
        child: Icon(Icons.add),
      ), // This trailing comma makes auto-formatting nicer for build methods.
    );
  }
}
