# openHT2

Свободная реализация движка **Hard Truck 2 / «Дальнобойщики 2» (King of the Road)** на Rust и wgpu.
Цель: 1:1 совместимость с оригиналом, как у openOMSI для OMSI 2.

> Проект независимый и неофициальный. Он не содержит кода и ассетов игры и не связан с её правообладателями.
> Для запуска нужна ваша лицензионная копия игры. Форматы разбираются clean-room, по файлам данных.

## Статус
Ранняя стадия. Сейчас готово:

- [x] `ht2-formats`: `.RES` (141/141 файлов Steam-версии), `.way` (41/41), `.b3d` (45/45), текстуры `.txr` (3D-вариант)
- [x] `ht2-tool`: `res-ls`, `way-ls`, `b3d-ls`, `txr-png` (экспорт текстур в PNG)
- [x] `ht2-view`: просмотрщик моделей на wgpu (орбитальная камера, снимки в PNG)
- [ ] террейн (`terrain*.raw`), сборка мира из локаций, дороги по `.way`
- [ ] UI-текстуры, `GAME.CNF`, `.sch`, `vehicle.tech`
- [ ] физика грузовиков, задания, торговля, ИИ, звук

Подробности: [docs/FORMATS.md](docs/FORMATS.md).

## Сборка
```
cargo build --release
cargo run --release -p ht2-tool -- res-ls "<папка игры>/ENV/aa.res"
cargo run --release -p ht2-tool -- way-ls "<папка игры>/ENV/aa.way"
cargo run --release -p ht2-tool -- txr-png "<папка игры>/ENV/aa.res" out/
cargo run --release -p ht2-view -- "<папка игры>/ENV/aa.b3d" --only Combinat
```
В просмотрщике: левая кнопка мыши вращает камеру, колесо приближает. Параметры `--shot файл.png` (снимок без окна), `--yaw`, `--pitch`, `--size ШxВ`.

## Благодарности
Структуры форматов частично сверены с MIT-описаниями [reRNR/RNRFileFormats](https://github.com/reRNR/RNRFileFormats). Код написан независимо.

## Сайт
Папка `site/` публикуется на GitHub Pages workflow-ом `.github/workflows/pages.yml`.
В настройках репозитория: Settings → Pages → Source: **GitHub Actions**.

## Лицензия
AGPL-3.0-only (как у openOMSI). Текст лицензии — в файле `LICENSE`.
