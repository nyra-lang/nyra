#!/usr/bin/env node
/**
 * Sync webDocs/nyra-skill.md → skills/skill.md for repo agents.
 * nyra-skill.md is the canonical standalone source (public docs URLs, no repo paths).
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SOURCE = path.join(ROOT, 'webDocs', 'nyra-skill.md');
const SKILLS_OUT = path.join(ROOT, 'skills', 'skill.md');

fs.copyFileSync(SOURCE, SKILLS_OUT);
console.log('skills/skill.md synced from webDocs/nyra-skill.md');
