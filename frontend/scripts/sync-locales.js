import fs from 'fs-extra';
import path from 'path';
import { fileURLToPath } from 'url';

// ES Module equivalent for __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Source is now two levels up from the script's location
const sourceDir = path.resolve(__dirname, '../../locales');
// Target is one level up, then into public/locales
const targetDir = path.resolve(__dirname, '../public/locales');

async function syncLocales() {
  try {
    // Ensure the target directory exists
    await fs.ensureDir(targetDir);

    // Copy the source directory to the target directory
    await fs.copy(sourceDir, targetDir, { overwrite: true });

    console.log('Successfully synced locales to public/locales');
  } catch (err) {
    console.error('Error syncing locales:', err);
    process.exit(1);
  }
}

syncLocales();