import React from 'react';
import Head from 'next/head';
import Editor from '@monaco-editor/react';
import { ToastContainer, toast } from 'react-toastify';
import LZString from 'lz-string';

import * as rox from '../node_modules/rox/rox';

class Index extends React.Component {
  constructor(props) {
    super(props);
    this.sourceRef = React.createRef();
    this.state = {
      source: 'print "Hello World!";',
      output: '',
      opcode: '',
      errors: [],
      decorations: [],
    }

    if (typeof window !== 'undefined') {
      window.setState = this.setState.bind(this);
      let source = this.getSourceFromURL();
      if (source) {
        this.state.source = source;
      }

      window.addEventListener('hashchange', (_e) => {
        let source = this.getSourceFromURL();
        if (source) {
          this.setState({ source: source, output: '', opcode: '', errors: [] })
        }
      });
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

  copySource = () => {
    let source = this.sourceRef.current.getValue();
    let compressed = LZString.compressToEncodedURIComponent(source);
    navigator.clipboard.writeText(window.location.host + '/#' + compressed);
    toast("Copied!");
  }

  getSourceFromURL = () => {
    let compressed = window.location.hash.substring(1);
    return LZString.decompressFromEncodedURIComponent(compressed);
  }

  render() {
    return (
      <div className='main'>
        <Head>
          <title>Rox Playground</title>
          <link rel="icon" href="favicon.svg" sizes="any" type="image/svg+xml" />
        </Head>
        <div className='navbar'>
          <a className='home'>Rox Playground</a>
          <a className='nav-member' onClick={this.run}>Run &nbsp; ▶</a>
          <a className='nav-member' onClick={this.copySource}>Share</a>
        </div>
        <ToastContainer />
        <div className='container' >
          <div className='source'>
            <Editor
              theme='vs-dark'
              onMount={(editor, _) => this.sourceRef.current = editor}
              options={{ glyphMargin: true }}
              value={this.props.source || this.state.source}
            />
          </div>
          <div className='opcode'>
            <Editor
              theme='vs-dark'
              options={{ lineNumbers: 'off', readOnly: true }}
              value={this.state.opcode}
            />
          </div>
          <div className='output'>
            <Editor
              theme='vs-dark'
              options={{ lineNumbers: 'off', readOnly: true, minimap: { enabled: false } }}
              value={this.state.output}
            />
          </div>
        </div>
      </div>
    );
  }
}

export default Index;
