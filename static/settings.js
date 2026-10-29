// Settings page: shows the volume while it is moved and plays a sound when one is chosen;
// the download button of the geolocation databases shows that it is working.
(function () {
  'use strict';
  var geoForm = document.getElementById('geo-download');
  var geoButton = document.querySelector('[data-geo-download]');
  if (geoForm && geoButton) {
    geoForm.addEventListener('submit', function () {
      geoButton.disabled = true;
      geoButton.textContent = geoButton.getAttribute('data-busy') || geoButton.textContent;
    });
  }
  var volume = document.getElementById('volume');
  var out = document.querySelector('[data-volume-out]');
  if (!volume) { return; }
  volume.addEventListener('input', function () { if (out) { out.textContent = volume.value + ' %'; } });
  var selects = document.querySelectorAll('[data-sound-select]');
  for (var i = 0; i < selects.length; i++) {
    selects[i].addEventListener('change', function (e) {
      if (window.openpacketmonSound) { window.openpacketmonSound(e.target.value, Number(volume.value)); }
    });
  }
})();
