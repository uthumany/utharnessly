#!/usr/bin/env node
import React from 'react';
import chalk from 'chalk';
import { render } from 'ink';
import { App } from './app.js';

process.on('uncaughtException', error => {
  process.stderr.write(`${chalk.red('utharness-ui:')} ${error.message}\n`);
  process.exitCode = 1;
});

process.on('unhandledRejection', reason => {
  process.stderr.write(`${chalk.red('utharness-ui:')} ${String(reason)}\n`);
  process.exitCode = 1;
});

const instance = render(<App />, { exitOnCtrlC: false });
void instance.waitUntilExit().then(() => process.exit(0));
