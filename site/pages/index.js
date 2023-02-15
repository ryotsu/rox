import React from 'react';
import Editor from '@monaco-editor/react';

import * as rox from '../node_modules/rox/rox';

class Index extends React.Component {
  constructor(props) {
    super(props);
    this.sourceRef = React.createRef();
    this.state = {
      source: 'print "Hello, World!";',
      output: '',
      opcode: '',
      errors: [],
      decorations: [],
    }

    if (typeof window !== 'undefined') {
      window.setState = this.setState.bind(this);
    }

  }

  run = () => {
    let source = this.sourceRef.current.getValue();
    this.setState({ source: source });
    rox.run(source);
  }

  componentDidUpdate(_, prevState) {
    if (prevState.source == this.state.source) {
      return;
    }

    let { errors, decorations } = this.state;

    decorations.forEach((decoration) => {
      this.sourceRef.current.deltaDecorations(
        [decoration], []
      );
    });

    let new_decorations = Array.from(errors).map((line) => {
      return this.sourceRef.current.deltaDecorations(
        [],
        [
          {
            range: new monaco.Range(line, 1, line, 1),
            options: {
              isWholeLine: true,
              className: 'myContentClass',
              glyphMarginClassName: "myGlyphMarginClass",
            }
          }
        ]
      );
    });

    this.setState({ decorations: new_decorations })
  }

  render() {
    return (
      <div className='container' >
        <button onClick={this.run}>Run</button>
        <Editor
          className='source'
          theme='vs-dark'
          onMount={(editor, _) => this.sourceRef.current = editor}
          options={{ glyphMargin: true }}
          value={this.state.source}
        />
        <Editor
          className='output'
          theme='vs-dark'
          options={{ lineNumbers: 'off', readOnly: true }}
          value={this.state.output}
        />
        <Editor
          className='opcode'
          theme='vs-dark'
          options={{ lineNumbers: 'off', readOnly: true }}
          value={this.state.opcode}
        />
      </div>
    );
  }
}

export default Index;
