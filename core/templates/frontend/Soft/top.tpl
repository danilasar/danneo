<!DOCTYPE html>
<html lang="{langcode}">
<head>   
<meta charset="{langcharset}" /> 
<title>{title}</title>
<meta name="distribution" content="global" />
<meta name="generator" content="CMS Danneo {version}" />
<meta name="author" content="{site}" />
<meta name="copyright" content="Copyright (c) {site}" />
<meta name="keywords" content="{keywords}" />
<meta name="description" content="{descript}" />
<script src="{site_url}/javascript/jquery.js"></script> 
<script src="{site_url}/javascript/jquery.colorbox.js"></script>
<script src="{site_url}/temp/{site_temp}/javascript/menu.js"></script>
<script src="{site_url}/temp/{site_temp}/javascript/soft.js"></script>
<link rel="stylesheet" href="{site_url}/temp/{site_temp}/css/screen.css" />
<link rel="stylesheet" href="{site_url}/temp/{site_temp}/css/menu.css" />  
<link rel="stylesheet" href="{site_url}/temp/{site_temp}/css/liquid-slider.css" /> 
<link rel="stylesheet" href="{site_url}/temp/{site_temp}/javascript/colorbox/colorbox.css" />
<link rel="alternate" type="application/rss+xml" title="{site}" href="{site_url}/rss.php?dn=news" /> 
<link rel="icon" href="{site_url}/favicon.ico" type="image/x-icon" />
<link rel="shortcut icon" href="{site_url}/favicon.ico" type="image/x-icon" /> 
<script>$(function(){ $.url = '{site_url}'; });</script>
</head>
<body>
<div id="logo">
 <div id="w1"><h1><a href="{site_url}/" title="{site}">{site}</a></h1></div>
</div>
<div id="top-menu">
  <div id="w2">{dnmtop}</div>
</div>
<div id="wrap">
<table id="table-main" cellspacing="0" cellpadding="0">
 <tbody>
  <tr>
   <td id="table-left">
    {leftblock}
   </td>
   <td id="table-center">
    {promoblock}{topblock}
    {breadcrumb}
