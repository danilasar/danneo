<!DOCTYPE html>
<html>
<head>
<meta charset="{langcharset}"> 
<title>{title}</title>
<link id="changer" rel="stylesheet" href="{site_url}/temp/{site_temp}/css/go.css" />   
<script>
    setTimeout(function() {
        window.location.href = '{url}';
    }, ({sec}*1000));
</script>
</head>
<body>  
<div>
    <p><big id="time"></big>
    {message}    
    <p class="ac">{link}</p>
</div>
<script>
    var line = {sec};
    timeline();
    function timeline() {
        if(line > 0) {
            document.getElementById('time').innerHTML = line;
            line = line - 1;
            setTimeout("timeline()",1000);
        } else {
            document.getElementById('time').innerHTML = '';
        }
    }
</script>
</body>
</html>
